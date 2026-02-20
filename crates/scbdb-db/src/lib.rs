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

pub mod bills;
pub mod brands;
pub mod collection_runs;
pub mod products;
pub mod seed;
pub mod sentiment;

pub use bills::{
    get_bill_by_jurisdiction_number, list_bill_events, list_bills, upsert_bill, upsert_bill_event,
    BillEventRow, BillRow,
};
pub use brands::{get_brand_by_slug, list_active_brands, BrandRow};
pub use collection_runs::{
    complete_collection_run, create_collection_run, fail_collection_run, get_collection_run,
    list_collection_run_brands, list_collection_runs, start_collection_run,
    upsert_collection_run_brand, CollectionRunBrandRow, CollectionRunRow,
};
pub use products::{
    get_last_price_snapshot, insert_price_snapshot_if_changed, upsert_product, upsert_variant,
    PriceSnapshotRow, ProductRow, VariantRow,
};
pub use sentiment::{
    get_latest_sentiment_by_brand, insert_sentiment_snapshot, list_sentiment_snapshots,
    SentimentSnapshotRow,
};
