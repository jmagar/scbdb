//! Background job scheduler.
//!
//! Initialises a [`JobScheduler`] at server startup and registers
//! recurring collection jobs.

mod brand_intel;

use std::sync::Arc;

use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

/// Builds and starts the background job scheduler.
///
/// Registers all recurring collection jobs and starts the scheduler.
/// Returns the running [`JobScheduler`] handle, which must be kept alive
/// for the lifetime of the process — dropping it shuts down all jobs.
///
/// # Errors
///
/// Returns [`JobSchedulerError`] if the scheduler cannot be initialised,
/// a job cannot be registered, or the scheduler fails to start.
pub async fn build_scheduler(
    pool: PgPool,
    config: Arc<scbdb_core::AppConfig>,
) -> Result<JobScheduler, JobSchedulerError> {
    let scheduler = JobScheduler::new().await?;

    register_locations_job(&scheduler, pool.clone(), Arc::clone(&config)).await?;
    brand_intel::register_signal_refresh_job(&scheduler, pool.clone()).await?;
    brand_intel::register_brand_intake_job(&scheduler, pool.clone()).await?;
    brand_intel::register_handle_refresh_job(&scheduler, pool).await?;

    scheduler.start().await?;
    Ok(scheduler)
}

/// Register a weekly store-locations collection job.
///
/// Runs every Sunday at 02:00 UTC (`0 0 2 * * SUN`). For each brand with a
/// `store_locator_url` the job fetches current store locations, upserts
/// new/changed records, and deactivates any that have gone missing.
async fn register_locations_job(
    scheduler: &JobScheduler,
    pool: PgPool,
    config: Arc<scbdb_core::AppConfig>,
) -> Result<(), JobSchedulerError> {
    let pool = Arc::new(pool);

    let job = Job::new_async("0 0 2 * * SUN", move |_uuid, _lock| {
        let pool = Arc::clone(&pool);
        let config = Arc::clone(&config);

        Box::pin(async move {
            tracing::info!("scheduler: starting weekly store-locations run");
            run_locations_job(&pool, &config).await;
            tracing::info!("scheduler: weekly store-locations run complete");
        })
    })?;

    scheduler.add(job).await?;
    Ok(())
}

/// Drive the store-locations collection for all brands with a locator URL.
async fn run_locations_job(pool: &PgPool, config: &scbdb_core::AppConfig) {
    let brands = match scbdb_db::list_brands_with_locator(pool).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "scheduler: failed to load brands with locator");
            return;
        }
    };

    if brands.is_empty() {
        tracing::info!("scheduler: no brands with store_locator_url; skipping");
        return;
    }

    tracing::info!(
        count = brands.len(),
        "scheduler: collecting locations for brands"
    );

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            config.scraper_request_timeout_secs,
        ))
        .build()
        .expect("failed to build HTTP client");

    for brand in &brands {
        let Some(locator_url) = &brand.store_locator_url else {
            // list_brands_with_locator guarantees Some; guard defensively.
            continue;
        };
        collect_brand_locations(pool, &http_client, config, brand, locator_url).await;
    }
}

/// Fetch, upsert, and deactivate locations for a single brand.
async fn collect_brand_locations(
    pool: &PgPool,
    client: &reqwest::Client,
    config: &scbdb_core::AppConfig,
    brand: &scbdb_db::BrandRow,
    locator_url: &str,
) {
    let raw = match scbdb_scraper::fetch_store_locations(
        client,
        locator_url,
        config.scraper_request_timeout_secs,
        &config.scraper_user_agent,
    )
    .await
    {
        Ok(locs) => locs,
        Err(e) => {
            // NOTE: a scrape failure aborts this entire brand — no partial results are
            // processed. This is intentional: partial scrape output is unreliable, and
            // deactivating locations based on incomplete data would be worse than skipping
            // the brand entirely. If partial-result handling is needed in the future,
            // the scraper should return a structured result distinguishing "no data" from
            // "partial data" so this function can decide accordingly.
            tracing::error!(brand = %brand.slug, error = %e, "scheduler: location scrape failed");
            return;
        }
    };

    let source = raw
        .first()
        .map_or("none", |loc| loc.locator_source.as_str());

    if let Err(reason) = scbdb_scraper::validate_store_locations_trust(&raw) {
        tracing::warn!(
            brand = %brand.slug,
            locator_url,
            source,
            "scheduler: rejected location scrape result ({reason})"
        );
        return;
    }

    // Guard: if the scrape returned zero locations, skip upsert and deactivation.
    // An empty result likely means a transient failure or a parse miss — deactivating
    // all existing locations would be destructive and incorrect.
    if raw.is_empty() {
        tracing::warn!(
            brand = %brand.slug,
            locator_url,
            "scheduler: scrape returned 0 locations; skipping upsert and deactivation"
        );
        return;
    }

    let new_locations: Vec<scbdb_db::NewStoreLocation> = raw
        .iter()
        .map(|loc| scbdb_db::NewStoreLocation {
            location_key: scbdb_scraper::make_location_key(brand.id, loc),
            name: loc.name.clone(),
            address_line1: loc.address_line1.clone(),
            city: loc.city.clone(),
            state: loc.state.clone(),
            zip: loc.zip.clone(),
            country: loc.country.clone().or_else(|| Some("US".to_string())),
            latitude: loc.latitude,
            longitude: loc.longitude,
            phone: loc.phone.clone(),
            external_id: loc.external_id.clone(),
            locator_source: Some(loc.locator_source.clone()),
            raw_data: loc.raw_data.clone(),
        })
        .collect();

    let active_keys: Vec<String> = new_locations
        .iter()
        .map(|l| l.location_key.clone())
        .collect();

    upsert_and_log(pool, brand, &new_locations).await;
    deactivate_and_log(pool, brand, &active_keys).await;
}

/// Upsert a batch of locations and log the result.
async fn upsert_and_log(
    pool: &PgPool,
    brand: &scbdb_db::BrandRow,
    locations: &[scbdb_db::NewStoreLocation],
) {
    match scbdb_db::upsert_store_locations(pool, brand.id, locations).await {
        Ok((new_count, kept_count)) => {
            tracing::info!(
                brand = %brand.slug,
                new = new_count,
                kept = kept_count,
                "scheduler: locations upserted"
            );
        }
        Err(e) => {
            tracing::error!(brand = %brand.slug, error = %e, "scheduler: db upsert failed");
        }
    }
}

/// Deactivate missing locations and log the result.
async fn deactivate_and_log(pool: &PgPool, brand: &scbdb_db::BrandRow, active_keys: &[String]) {
    match scbdb_db::deactivate_missing_locations(pool, brand.id, active_keys).await {
        Ok(n) if n > 0 => {
            tracing::info!(
                brand = %brand.slug,
                deactivated = n,
                "scheduler: deactivated missing locations"
            );
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(
                brand = %brand.slug,
                error = %e,
                "scheduler: failed to deactivate missing locations"
            );
        }
    }
}
