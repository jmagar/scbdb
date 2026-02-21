//! Store location collection handler for the CLI.
//!
//! Orchestrates the full locations collection pipeline:
//! auto-discovers locator URLs for brands that don't have one configured,
//! fetches and parses raw store records from each brand's locator, and
//! persists active locations while deactivating stale ones.

mod brand;
mod helpers;
mod url;

use futures::stream::{self, StreamExt};

use crate::fail_run_best_effort;
use brand::collect_brand_locations;
use helpers::load_brands_for_locations;

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
        anyhow::bail!("no eligible brands found for location collection");
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
