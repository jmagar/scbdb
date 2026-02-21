use scbdb_core::AppConfig;
use sqlx::migrate::Migrate;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use thiserror::Error;

// Path relative to crates/scbdb-db/Cargo.toml; resolves to <workspace-root>/migrations/
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, Clone, Copy)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_secs: 10,
        }
    }
}

impl PoolConfig {
    #[must_use]
    pub fn from_app_config(config: &AppConfig) -> Self {
        Self {
            max_connections: config.db_max_connections,
            min_connections: config.db_min_connections,
            acquire_timeout_secs: config.db_acquire_timeout_secs,
        }
    }
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("DATABASE_URL is not set")]
    MissingDatabaseUrl,
    #[error("record not found")]
    NotFound,
    #[error(
        "invalid collection run state transition for id {id}: expected status '{expected_status}'"
    )]
    InvalidCollectionRunTransition {
        id: i64,
        expected_status: &'static str,
    },
    #[error(transparent)]
    Config(#[from] scbdb_core::ConfigError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
}

/// Connect to a Postgres pool using explicit URL and config.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the connection cannot be established.
pub async fn connect_pool(database_url: &str, config: PoolConfig) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
        .connect(database_url)
        .await
}

/// Connect to a Postgres pool, reading `DATABASE_URL` and pool settings from env.
///
/// # Errors
///
/// Returns [`DbError::Config`] if configuration is missing/invalid, or
/// [`DbError::Sqlx`] if the connection cannot be established.
pub async fn connect_pool_from_env() -> Result<PgPool, DbError> {
    let app_config = scbdb_core::load_app_config_from_env()?;
    let pool_config = PoolConfig::from_app_config(&app_config);
    connect_pool(&app_config.database_url, pool_config)
        .await
        .map_err(DbError::from)
}

/// Run all pending migrations against the pool.
///
/// Returns the number of migrations that were applied.
///
/// # Errors
///
/// Returns [`sqlx::migrate::MigrateError`] if any migration fails.
pub async fn run_migrations(pool: &PgPool) -> Result<usize, sqlx::migrate::MigrateError> {
    let applied_before = {
        let mut conn = pool.acquire().await?;
        conn.ensure_migrations_table().await?;
        conn.list_applied_migrations().await?.len()
    };

    MIGRATOR.run(pool).await?;

    let applied_after = {
        let mut conn = pool.acquire().await?;
        conn.ensure_migrations_table().await?;
        conn.list_applied_migrations().await?.len()
    };

    Ok(applied_after.saturating_sub(applied_before))
}

/// Send a `SELECT 1` to verify the pool has a live connection.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}

/// Run a full health check: ping the pool and return a typed error on failure.
///
/// # Errors
///
/// Returns [`DbError`] if the ping fails.
pub async fn health_check(pool: &PgPool) -> Result<(), DbError> {
    ping(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_config_has_sane_defaults() {
        let config = PoolConfig::default();

        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.acquire_timeout_secs, 10);
    }
}

pub mod api_queries;
pub mod bill_events;
pub mod bill_texts;
pub mod bills;
pub mod brand_completeness;
pub mod brand_intel;
pub mod brand_profiles;
pub mod brand_signals;
pub mod brands;
pub mod collection_runs;
pub mod locations;
pub mod products;
pub mod seed;
pub mod sentiment;

pub use brand_completeness::{
    get_all_brands_completeness, get_brand_completeness, BrandCompletenessScore,
};
pub use brand_intel::{
    insert_brand_competitor_relationship, insert_brand_distributor, insert_brand_funding_event,
    insert_brand_lab_test, insert_brand_legal_proceeding, insert_brand_media_appearance,
    insert_brand_newsletter, insert_brand_sponsorship, list_brand_competitor_relationships,
    list_brand_distributors, list_brand_funding_events, list_brand_lab_tests,
    list_brand_legal_proceedings, list_brand_media_appearances, list_brand_newsletters,
    list_brand_sponsorships, BrandCompetitorRelationshipRow, BrandDistributorRow,
    BrandFundingEventRow, BrandLabTestRow, BrandLegalProceedingRow, BrandMediaAppearanceRow,
    BrandNewsletterRow, BrandSponsorshipRow, NewBrandCompetitorRelationship, NewBrandDistributor,
    NewBrandFundingEvent, NewBrandLabTest, NewBrandLegalProceeding, NewBrandMediaAppearance,
    NewBrandNewsletter, NewBrandSponsorship,
};
pub use brand_signals::{
    list_brand_feed_urls, list_brand_signals, list_brands_needing_signal_refresh,
    list_brands_with_stale_handles, upsert_brand_signal, BrandSignalRow, NewBrandSignal,
};

pub use api_queries::{
    list_price_snapshots_dashboard, list_pricing_summary, list_products_dashboard,
    list_sentiment_snapshots_dashboard, list_sentiment_summary, PriceSnapshotDashboardRow,
    PriceSnapshotFilters, PricingSummaryRow, ProductDashboardRow, ProductListFilters,
    SentimentSnapshotDashboardRow, SentimentSummaryRow,
};

pub use bill_events::{
    list_bill_events, list_bill_events_batch, list_bill_events_by_public_id, upsert_bill_event,
    BillEventRow,
};
pub use bill_texts::{
    get_bills_stored_hashes, list_bill_texts_by_public_id, upsert_bill_text, BillTextRow,
};
pub use bills::{
    get_bill_by_jurisdiction_number, get_bill_by_public_id, list_bills, upsert_bill, BillRow,
};
pub use brand_profiles::{
    get_brand_profile, list_brand_social_handles, list_brands_without_profiles,
    overwrite_brand_profile, replace_brand_domains, replace_brand_social_handles,
    upsert_brand_profile, BrandProfileRow, BrandSocialHandleRow,
};
pub use brands::{
    create_brand, deactivate_brand, get_brand_by_slug, list_active_brands,
    list_brands_with_locator, update_brand, update_brand_logo, update_brand_store_locator_url,
    BrandRow,
};
pub use collection_runs::{
    complete_collection_run, create_collection_run, fail_collection_run, get_collection_run,
    list_collection_run_brands, list_collection_runs, start_collection_run,
    upsert_collection_run_brand, CollectionRunBrandRow, CollectionRunRow,
};
pub use locations::{
    deactivate_missing_locations, get_active_location_keys_for_brand, list_active_location_pins,
    list_active_locations_by_brand, list_locations_by_state, list_locations_dashboard_summary,
    list_new_locations_since, upsert_store_locations, LocationPinRow, LocationsByStateRow,
    LocationsDashboardRow, NewStoreLocation, StoreLocationRow,
};
pub use products::{
    get_last_price_snapshot, insert_price_snapshot_if_changed, upsert_product, upsert_variant,
    PriceSnapshotRow, ProductRow, VariantRow,
};
pub use seed::{upsert_brand_domains, upsert_brand_social_handles};
pub use sentiment::{
    get_latest_sentiment_by_brand, insert_sentiment_snapshot, list_sentiment_snapshots,
    SentimentSnapshotRow,
};
