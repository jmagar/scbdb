//! Per-brand location collection logic.

use helpers::{raw_to_new_location, record_brand_failure};
use url::resolve_locator_url;

use super::helpers;
use super::url;
use super::BrandLocationOutcome;

/// Fetch and persist store locations for a single brand.
///
/// Always returns a `BrandLocationOutcome`; errors are captured inside
/// the outcome rather than propagated, so a failing brand does not abort
/// the whole run.
#[allow(clippy::too_many_lines)] // Orchestration function: URL resolve, scrape, upsert, deactivate, audit
pub(super) async fn collect_brand_locations(
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

    if let Err(reason) = scbdb_scraper::validate_store_locations_trust(&raw_locations) {
        let err_msg = format!("untrusted scrape result: {reason}");
        tracing::warn!(
            brand = %brand.slug,
            locator_url = %locator_url,
            source = source.as_deref().unwrap_or("none"),
            "location scrape rejected: {err_msg}"
        );
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

    // Snapshot active keys before upsert for diff logging.
    let prev_keys: std::collections::HashSet<String> =
        match scbdb_db::get_active_location_keys_for_brand(pool, brand.id).await {
            Ok(keys) => keys,
            Err(e) => {
                tracing::warn!(brand = %brand.slug, error = %e, "could not snapshot active keys; diff logging skipped");
                std::collections::HashSet::new()
            }
        };

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

    // Post-upsert diff: compare current scrape key set against pre-upsert snapshot.
    let curr_keys: std::collections::HashSet<String> = active_keys.iter().cloned().collect();
    let added_count = curr_keys.difference(&prev_keys).count();
    let removed_count = prev_keys.difference(&curr_keys).count();
    if added_count > 0 {
        tracing::info!(brand = %brand.slug, count = added_count, "new store locations detected");
    }
    if removed_count > 0 {
        tracing::info!(brand = %brand.slug, count = removed_count, "store locations deactivated");
    }

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
