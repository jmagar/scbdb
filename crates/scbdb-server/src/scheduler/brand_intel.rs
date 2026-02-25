//! Scheduled jobs for the brand intelligence layer.
//!
//! Registers brand intake, signal refresh, and handle refresh jobs
//! that run the profiler pipeline on a recurring schedule.

use std::sync::Arc;

use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

/// Register a daily brand intake job.
///
/// Runs at 06:00 UTC by default (`0 0 6 * * *`) and can be overridden with
/// `BRAND_INTAKE_CRON`.
pub(super) async fn register_brand_intake_job(
    scheduler: &JobScheduler,
    pool: PgPool,
) -> Result<(), JobSchedulerError> {
    let cron = std::env::var("BRAND_INTAKE_CRON").unwrap_or_else(|_| "0 0 6 * * *".to_string());
    let tei_url: Arc<str> = std::env::var("TEI_URL").unwrap_or_default().into();
    let youtube_api_key: Option<Arc<str>> = std::env::var("YOUTUBE_API_KEY").ok().map(Into::into);
    let pool = Arc::new(pool);
    let client = reqwest::Client::new();

    let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
        let pool = Arc::clone(&pool);
        let client = client.clone();
        let tei_url = Arc::clone(&tei_url);
        let youtube_api_key = youtube_api_key.clone();

        Box::pin(async move {
            tracing::info!("scheduler: starting daily brand_intake run");
            run_brand_intake_job(&pool, &client, &tei_url, youtube_api_key.as_deref()).await;
            tracing::info!("scheduler: daily brand_intake run complete");
        })
    })?;

    scheduler.add(job).await?;
    tracing::info!(cron = %cron, "scheduler: registered brand_intake job");
    Ok(())
}

/// Drive the brand intake pipeline for brands missing profiles.
///
/// For each brand without a profile, loads social handles and domain feed URLs,
/// then runs the profiler intake pipeline. Individual brand failures are logged
/// but do not abort the run.
async fn run_brand_intake_job(
    pool: &PgPool,
    client: &reqwest::Client,
    tei_url: &str,
    youtube_api_key: Option<&str>,
) {
    let brand_ids = match scbdb_db::list_brands_without_profiles(pool).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(error = %e, "scheduler: brand_intake failed to list brands");
            return;
        }
    };

    if brand_ids.is_empty() {
        tracing::info!("scheduler: brand_intake: all brands have profiles; skipping");
        return;
    }

    tracing::info!(
        count = brand_ids.len(),
        "scheduler: brand_intake: processing brands without profiles"
    );

    for brand_id in &brand_ids {
        run_brand_intake_for(*brand_id, pool, client, tei_url, youtube_api_key).await;
    }
}

/// Run the intake pipeline for a single brand.
///
/// Loads the brand's feed URLs and social handles, then invokes the profiler.
/// All errors are logged rather than propagated so one brand's failure does not
/// block the rest of the batch.
async fn run_brand_intake_for(
    brand_id: i64,
    pool: &PgPool,
    client: &reqwest::Client,
    tei_url: &str,
    youtube_api_key: Option<&str>,
) {
    // Load feed URLs from brand_domains
    let feed_urls = match scbdb_db::list_brand_feed_urls(pool, brand_id).await {
        Ok(urls) => urls,
        Err(e) => {
            tracing::warn!(brand_id, error = %e, "scheduler: brand_intake: failed to load feed URLs");
            Vec::new()
        }
    };

    // Load social handles to find youtube_channel_id and twitter_handle
    let handles = match scbdb_db::list_brand_social_handles(pool, brand_id).await {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(brand_id, error = %e, "scheduler: brand_intake: failed to load social handles");
            Vec::new()
        }
    };

    let youtube_channel_id = handles
        .iter()
        .find(|h| h.platform == "youtube")
        .map(|h| h.handle.clone());

    let twitter_handle = handles
        .iter()
        .find(|h| h.platform == "twitter")
        .map(|h| h.handle.clone());

    let intake_config = scbdb_profiler::IntakeConfig {
        client: client.clone(),
        tei_url: tei_url.to_string(),
        youtube_api_key: youtube_api_key.map(String::from),
    };

    match scbdb_profiler::intake::ingest_signals(
        pool,
        &intake_config,
        brand_id,
        &feed_urls,
        youtube_channel_id.as_deref(),
        twitter_handle.as_deref(),
    )
    .await
    {
        Ok(result) => {
            tracing::info!(
                brand_id,
                signals_collected = result.signals_collected,
                signals_upserted = result.signals_upserted,
                error_count = result.errors.len(),
                "scheduler: brand_intake: completed for brand"
            );
        }
        Err(e) => {
            tracing::error!(brand_id, error = %e, "scheduler: brand_intake: fatal error for brand");
        }
    }
}

/// Register a daily signal refresh job.
///
/// Runs every day at 04:00 UTC (`0 0 4 * * *`).
pub(super) async fn register_signal_refresh_job(
    scheduler: &JobScheduler,
    pool: PgPool,
) -> Result<(), JobSchedulerError> {
    let tei_url: Arc<str> = std::env::var("TEI_URL").unwrap_or_default().into();
    let youtube_api_key: Option<Arc<str>> = std::env::var("YOUTUBE_API_KEY").ok().map(Into::into);
    let pool = Arc::new(pool);
    let client = reqwest::Client::new();

    let job = Job::new_async("0 0 4 * * *", move |_uuid, _lock| {
        let pool = Arc::clone(&pool);
        let client = client.clone();
        let tei_url = Arc::clone(&tei_url);
        let youtube_api_key = youtube_api_key.clone();

        Box::pin(async move {
            tracing::info!("scheduler: starting daily signal_refresh run");
            run_signal_refresh_job(&pool, &client, &tei_url, youtube_api_key.as_deref()).await;
            tracing::info!("scheduler: daily signal_refresh run complete");
        })
    })?;

    scheduler.add(job).await?;
    tracing::info!("scheduler: registered signal_refresh job (daily 04:00 UTC)");
    Ok(())
}

/// Identify brands with stale signals (>24 hours) and re-run intake for each.
async fn run_signal_refresh_job(
    pool: &PgPool,
    client: &reqwest::Client,
    tei_url: &str,
    youtube_api_key: Option<&str>,
) {
    let stale_hours = 24;

    let brand_ids = match scbdb_db::list_brands_needing_signal_refresh(pool, stale_hours).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(error = %e, "scheduler: signal_refresh failed to query stale brands");
            return;
        }
    };

    if brand_ids.is_empty() {
        tracing::info!("scheduler: signal_refresh: all brands up-to-date; skipping");
        return;
    }

    tracing::info!(
        count = brand_ids.len(),
        stale_hours,
        "scheduler: signal_refresh: brands needing refresh"
    );

    for brand_id in &brand_ids {
        run_brand_intake_for(*brand_id, pool, client, tei_url, youtube_api_key).await;
    }
}

/// Register a weekly social-handle verification job.
///
/// Runs every Sunday at 05:00 UTC (`0 0 5 * * SUN`).
pub(super) async fn register_handle_refresh_job(
    scheduler: &JobScheduler,
    pool: PgPool,
) -> Result<(), JobSchedulerError> {
    let pool = Arc::new(pool);

    let job = Job::new_async("0 0 5 * * SUN", move |_uuid, _lock| {
        let pool = Arc::clone(&pool);

        Box::pin(async move {
            tracing::info!("scheduler: starting weekly handle_refresh run");
            run_handle_refresh_job(&pool).await;
            tracing::info!("scheduler: weekly handle_refresh run complete");
        })
    })?;

    scheduler.add(job).await?;
    tracing::info!("scheduler: registered handle_refresh job (weekly Sunday 05:00 UTC)");
    Ok(())
}

/// Check for brands with stale social handles and update `last_checked_at`.
///
/// Loads brands whose handles haven't been verified in 7+ days and logs them.
/// Full handle verification (checking profile URLs, follower counts) is a
/// future enhancement; for now this marks the handles as checked.
async fn run_handle_refresh_job(pool: &PgPool) {
    let stale_days = 7;

    let brand_ids = match scbdb_db::list_brands_with_stale_handles(pool, stale_days).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(error = %e, "scheduler: handle_refresh failed to query stale handles");
            return;
        }
    };

    if brand_ids.is_empty() {
        tracing::info!("scheduler: handle_refresh: all handles recently checked; skipping");
        return;
    }

    tracing::info!(
        count = brand_ids.len(),
        stale_days,
        "scheduler: handle_refresh: brands with stale handles"
    );

    // Load and log handles per brand. Full verification (HTTP checks,
    // follower-count refresh) will be added once the handle-verification
    // service is implemented.
    for brand_id in &brand_ids {
        match scbdb_db::list_brand_social_handles(pool, *brand_id).await {
            Ok(handles) => {
                let platforms: Vec<&str> = handles.iter().map(|h| h.platform.as_str()).collect();
                tracing::info!(
                    brand_id,
                    handle_count = handles.len(),
                    ?platforms,
                    "scheduler: handle_refresh: stale handles found"
                );
                // TODO: verify each handle URL, refresh follower counts,
                // and update last_checked_at once the verification service exists.
            }
            Err(e) => {
                tracing::warn!(
                    brand_id,
                    error = %e,
                    "scheduler: handle_refresh: failed to load handles"
                );
            }
        }
    }
}
