use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, time::Duration};
use thiserror::Error;

const DEFAULT_MAX_CONNECTIONS: u32 = 10;
const DEFAULT_MIN_CONNECTIONS: u32 = 1;
const DEFAULT_ACQUIRE_TIMEOUT_SECS: u64 = 10;

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
            max_connections: DEFAULT_MAX_CONNECTIONS,
            min_connections: DEFAULT_MIN_CONNECTIONS,
            acquire_timeout_secs: DEFAULT_ACQUIRE_TIMEOUT_SECS,
        }
    }
}

impl PoolConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            max_connections: read_u32("SCBDB_DB_MAX_CONNECTIONS", DEFAULT_MAX_CONNECTIONS),
            min_connections: read_u32("SCBDB_DB_MIN_CONNECTIONS", DEFAULT_MIN_CONNECTIONS),
            acquire_timeout_secs: read_u64(
                "SCBDB_DB_ACQUIRE_TIMEOUT_SECS",
                DEFAULT_ACQUIRE_TIMEOUT_SECS,
            ),
        }
    }
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("DATABASE_URL is not set")]
    MissingDatabaseUrl,
    #[error("record not found")]
    NotFound,
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
/// Returns [`DbError::MissingDatabaseUrl`] if `DATABASE_URL` is unset, or
/// [`DbError::Sqlx`] if the connection cannot be established.
pub async fn connect_pool_from_env() -> Result<PgPool, DbError> {
    let database_url = env::var("DATABASE_URL").map_err(|_| DbError::MissingDatabaseUrl)?;
    let config = PoolConfig::from_env();
    connect_pool(&database_url, config)
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
    // Count applied migrations before running. The _sqlx_migrations table may not
    // exist yet on a fresh database; treat absence as zero applied.
    let applied_before: i64 =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = true")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    MIGRATOR.run(pool).await?;

    let applied_after: i64 =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = true")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let delta = (applied_after - applied_before).max(0);
    Ok(usize::try_from(delta).unwrap_or(0))
}

/// Send a `SELECT 1` to verify the pool has a live connection.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
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

fn read_u32(var: &str, default: u32) -> u32 {
    env::var(var)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(default)
}

fn read_u64(var: &str, default: u64) -> u64 {
    env::var(var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_config_has_sane_defaults() {
        let config = PoolConfig::default();

        assert_eq!(config.max_connections, DEFAULT_MAX_CONNECTIONS);
        assert_eq!(config.min_connections, DEFAULT_MIN_CONNECTIONS);
        assert_eq!(config.acquire_timeout_secs, DEFAULT_ACQUIRE_TIMEOUT_SECS);
    }
}

pub mod brands;
pub mod collection_runs;
pub mod products;
pub mod seed;

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
