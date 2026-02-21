//! Store location collection handler for the CLI.
//!
//! Orchestrates the full locations collection pipeline:
//! auto-discovers locator URLs for brands that don't have one configured,
//! fetches and parses raw store records from each brand's locator, and
//! persists active locations while deactivating stale ones.

mod helpers;
mod url;

use futures::stream::{self, StreamExt};

use crate::fail_run_best_effort;
use helpers::{load_brands_for_locations, raw_to_new_location, record_brand_failure};
use url::resolve_locator_url;

// ---------------------------------------------------------------------------
// Outcome types
// ---------------------------------------------------------------------------

/// Result of processing a single brand during a locations collection run.
struct BrandLocationOutcome {
    /// Number of currently-active locations after the run.
    active: i64,
    /// Locations that did not exist before this run.
    new: i64,
    /// Locations deactivated because they were absent from the locator.
    lost: i64,
    /// Which extraction strategy produced the records.
    source: Option<String>,
    /// Whether the brand completed without a fatal error.
    succeeded: bool,
    /// Human-readable error description, set when `succeeded` is `false`.
    error: Option<String>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Collect store retail locations for all active brands.
///
/// For each brand, the function:
/// 1. Resolves the locator URL from `brands.store_locator_url` or via
///    HTTP auto-discovery over the brand's domain.
/// 2. Fetches and parses raw store records.
/// 3. Upserts active locations and deactivates missing ones.
///
/// When `dry_run` is `true`, prints what would be attempted and returns
/// without touching the database.
///
/// # Errors
///
/// Returns an error if the brand filter resolves to nothing or the
/// collection run cannot be created.  Per-brand failures are logged and
/// skipped, not propagated.
pub(crate) async fn run_collect_locations(
    pool: &sqlx::PgPool,
    config: &scbdb_core::AppConfig,
    brand_filter: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let brands = load_brands_for_locations(pool, brand_filter).await?;

    if brands.is_empty() {
        println!("no eligible brands found for location collection; skipping run creation");
        return Ok(());
    }

    if dry_run {
        println!(
            "dry-run: would attempt location collection for {} brand(s):",
            brands.len()
        );
        for brand in &brands {
            let url_hint = brand.store_locator_url.as_deref().map_or_else(
                || "auto-discover".to_string(),
                |u| format!("configured ({u})"),
            );
            println!("  {} — {url_hint}", brand.slug);
        }
        return Ok(());
    }

    println!("Collecting store locations for {} brands...", brands.len());

    let run = scbdb_db::create_collection_run(pool, "locations", "cli").await?;
    if let Err(e) = scbdb_db::start_collection_run(pool, run.id).await {
        fail_run_best_effort(pool, run.id, "locations", format!("{e:#}")).await;
        return Err(e.into());
    }

    let max_concurrent = config.scraper_max_concurrent_brands.max(1);

    let results: Vec<(&scbdb_db::BrandRow, BrandLocationOutcome)> = stream::iter(&brands)
        .map(|brand| {
            let fut = collect_brand_locations(pool, config, run.id, brand);
            async move { (brand, fut.await) }
        })
        .buffer_unordered(max_concurrent)
        .collect()
        .await;

    let mut total_active: i64 = 0;
    let mut total_new: i64 = 0;
    let mut failed_brands: usize = 0;

    for (brand, outcome) in &results {
        if outcome.succeeded {
            total_active = total_active.saturating_add(outcome.active);
            total_new = total_new.saturating_add(outcome.new);

            let source_label = outcome.source.as_deref().unwrap_or("none").to_string();

            println!(
                "  \u{2713} {:<20} {:>4} active ({:+} new, {} lost)  [{}]",
                brand.slug, outcome.active, outcome.new, outcome.lost, source_label,
            );
        } else {
            failed_brands += 1;
            let err_msg = outcome.error.as_deref().unwrap_or("unknown error");
            println!("  \u{2717} {:<20} {}", brand.slug, err_msg,);
        }
    }

    let brand_count = brands.len();

    if failed_brands == brand_count {
        let message = format!("all {failed_brands} brands failed location collection");
        fail_run_best_effort(pool, run.id, "locations", message.clone()).await;
        anyhow::bail!("{message}");
    }

    let total_as_i32 = i32::try_from(total_active).unwrap_or(i32::MAX);
    if let Err(err) = scbdb_db::complete_collection_run(pool, run.id, total_as_i32).await {
        let message = format!("{err:#}");
        fail_run_best_effort(pool, run.id, "locations", message.clone()).await;
        return Err(err.into());
    }

    println!("Run complete: {total_active} total active locations, {total_new} new this run");

    Ok(())
}

// ---------------------------------------------------------------------------
// Per-brand processing
// ---------------------------------------------------------------------------

/// Fetch and persist store locations for a single brand.
///
/// Always returns a `BrandLocationOutcome`; errors are captured inside
/// the outcome rather than propagated, so a failing brand does not abort
/// the whole run.
#[allow(clippy::too_many_lines)] // Orchestration function: URL resolve, scrape, upsert, deactivate, audit
async fn collect_brand_locations(
    pool: &sqlx::PgPool,
    config: &scbdb_core::AppConfig,
    run_id: i64,
    brand: &scbdb_db::BrandRow,
) -> BrandLocationOutcome {
    let Some(locator_url) = resolve_locator_url(brand, config).await else {
        let err_msg = "no locator URL configured or discovered".to_string();
        tracing::warn!(brand = %brand.slug, "{err_msg}");
        record_brand_failure(pool, run_id, brand, &err_msg).await;
        return BrandLocationOutcome {
            active: 0,
            new: 0,
            lost: 0,
            source: None,
            succeeded: false,
            error: Some(err_msg),
        };
    };

    let raw_locations = match scbdb_scraper::fetch_store_locations(
        &locator_url,
        config.scraper_request_timeout_secs,
        &config.scraper_user_agent,
    )
    .await
    {
        Ok(locs) => locs,
        Err(e) => {
            let err_msg = format!("scrape failed: {e}");
            tracing::error!(brand = %brand.slug, error = %e, "location scrape failed");
            record_brand_failure(pool, run_id, brand, &err_msg).await;
            return BrandLocationOutcome {
                active: 0,
                new: 0,
                lost: 0,
                source: None,
                succeeded: false,
                error: Some(err_msg),
            };
        }
    };

    let source: Option<String> = raw_locations.first().map(|loc| loc.locator_source.clone());

    let new_locations: Vec<scbdb_db::NewStoreLocation> = raw_locations
        .iter()
        .map(|loc| {
            let key = scbdb_scraper::make_location_key(brand.id, loc);
            raw_to_new_location(loc, key)
        })
        .collect();

    let active_keys: Vec<String> = new_locations
        .iter()
        .map(|l| l.location_key.clone())
        .collect();

    let (new_count, kept_count) =
        match scbdb_db::upsert_store_locations(pool, brand.id, &new_locations).await {
            Ok(counts) => counts,
            Err(e) => {
                let err_msg = format!("db error upserting locations: {e:#}");
                tracing::error!(brand = %brand.slug, error = %e, "db upsert failed");
                record_brand_failure(pool, run_id, brand, &err_msg).await;
                return BrandLocationOutcome {
                    active: 0,
                    new: 0,
                    lost: 0,
                    source,
                    succeeded: false,
                    error: Some(err_msg),
                };
            }
        };

    let lost_count = match scbdb_db::deactivate_missing_locations(pool, brand.id, &active_keys)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(brand = %brand.slug, error = %e, "failed to deactivate missing locations");
            0
        }
    };

    let total_active = new_count.saturating_add(kept_count);
    let records_i32 = i32::try_from(total_active).unwrap_or(i32::MAX);

    if let Err(e) = scbdb_db::upsert_collection_run_brand(
        pool,
        run_id,
        brand.id,
        "succeeded",
        Some(records_i32),
        None,
    )
    .await
    {
        tracing::error!(
            brand = %brand.slug,
            run_id,
            error = %e,
            "location data saved but failed to record brand success — audit trail incomplete"
        );
    }

    BrandLocationOutcome {
        active: i64::from(records_i32),
        new: i64::try_from(new_count).unwrap_or(i64::MAX),
        lost: i64::try_from(lost_count).unwrap_or(i64::MAX),
        source,
        succeeded: true,
        error: None,
    }
}
