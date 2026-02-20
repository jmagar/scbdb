//! Database operations for `collection_runs` and `collection_run_brands`.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::DbError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

/// A row from the `collection_runs` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CollectionRunRow {
    pub id: i64,
    pub public_id: Uuid,
    pub run_type: String,
    pub trigger_source: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    /// The schema defines this as `INTEGER NOT NULL DEFAULT 0`.
    pub records_processed: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A row from the `collection_run_brands` table.
///
/// Note: the schema does not include an `updated_at` column on this table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CollectionRunBrandRow {
    pub id: i64,
    pub collection_run_id: i64,
    pub brand_id: i64,
    pub status: String,
    /// The schema defines this as `INTEGER NOT NULL DEFAULT 0`.
    pub records_processed: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// collection_runs operations
// ---------------------------------------------------------------------------

/// Creates a new collection run in `queued` status.
///
/// Generates a UUID in Rust and binds it to `public_id`. Returns the full
/// newly-created row.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the insert or fetch fails.
pub async fn create_collection_run(
    pool: &PgPool,
    run_type: &str,
    trigger_source: &str,
) -> Result<CollectionRunRow, DbError> {
    let public_id = Uuid::new_v4();

    let row = sqlx::query_as::<_, CollectionRunRow>(
        "INSERT INTO collection_runs (public_id, run_type, trigger_source, status) \
         VALUES ($1, $2, $3, 'queued') \
         RETURNING id, public_id, run_type, trigger_source, status, \
                   started_at, completed_at, records_processed, error_message, created_at",
    )
    .bind(public_id)
    .bind(run_type)
    .bind(trigger_source)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Marks a run as `running` and sets `started_at = NOW()`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the update fails.
pub async fn start_collection_run(pool: &PgPool, id: i64) -> Result<(), DbError> {
    let result = sqlx::query(
        "UPDATE collection_runs \
         SET status = 'running', started_at = NOW() \
         WHERE id = $1 AND status = 'queued'",
    )
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::InvalidCollectionRunTransition {
            id,
            expected_status: "queued",
        });
    }

    Ok(())
}

/// Marks a run as `succeeded`, sets `completed_at = NOW()` and `records_processed`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the update fails.
pub async fn complete_collection_run(
    pool: &PgPool,
    id: i64,
    records_processed: i32,
) -> Result<(), DbError> {
    let result = sqlx::query(
        "UPDATE collection_runs \
         SET status = 'succeeded', completed_at = NOW(), records_processed = $1 \
         WHERE id = $2 AND status = 'running'",
    )
    .bind(records_processed)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::InvalidCollectionRunTransition {
            id,
            expected_status: "running",
        });
    }

    Ok(())
}

/// Marks a run as `failed`, sets `completed_at = NOW()` and `error_message`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the update fails.
pub async fn fail_collection_run(
    pool: &PgPool,
    id: i64,
    error_message: &str,
) -> Result<(), DbError> {
    let result = sqlx::query(
        "UPDATE collection_runs \
         SET status = 'failed', completed_at = NOW(), error_message = $1 \
         WHERE id = $2 AND status = 'running'",
    )
    .bind(error_message)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::InvalidCollectionRunTransition {
            id,
            expected_status: "running",
        });
    }

    Ok(())
}

/// Fetches a single run by its internal `id`.
///
/// # Errors
///
/// Returns [`DbError::NotFound`] if no row exists with the given `id`, or
/// [`DbError::Sqlx`] if the query fails.
pub async fn get_collection_run(pool: &PgPool, id: i64) -> Result<CollectionRunRow, DbError> {
    let row = sqlx::query_as::<_, CollectionRunRow>(
        "SELECT id, public_id, run_type, trigger_source, status, \
                started_at, completed_at, records_processed, error_message, created_at \
         FROM collection_runs \
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    Ok(row)
}

/// Returns the most recent `limit` runs, ordered by `created_at DESC`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_collection_runs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<CollectionRunRow>, DbError> {
    let rows = sqlx::query_as::<_, CollectionRunRow>(
        "SELECT id, public_id, run_type, trigger_source, status, \
                started_at, completed_at, records_processed, error_message, created_at \
         FROM collection_runs \
         ORDER BY created_at DESC, id DESC \
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

// ---------------------------------------------------------------------------
// collection_run_brands operations
// ---------------------------------------------------------------------------

/// Inserts or updates the per-brand result row for a collection run.
///
/// Conflicts on `(collection_run_id, brand_id)` update `status`,
/// `records_processed`, and `error_message` in place.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the upsert fails.
pub async fn upsert_collection_run_brand(
    pool: &PgPool,
    run_id: i64,
    brand_id: i64,
    status: &str,
    records_processed: Option<i32>,
    error_message: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO collection_run_brands \
             (collection_run_id, brand_id, status, records_processed, error_message) \
         VALUES ($1, $2, $3, COALESCE($4, 0), $5) \
         ON CONFLICT (collection_run_id, brand_id) DO UPDATE SET \
             status           = EXCLUDED.status, \
             records_processed = EXCLUDED.records_processed, \
             error_message    = EXCLUDED.error_message",
    )
    .bind(run_id)
    .bind(brand_id)
    .bind(status)
    .bind(records_processed)
    .bind(error_message)
    .execute(pool)
    .await?;

    Ok(())
}

/// Returns all brand-level result rows for a given collection run.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_collection_run_brands(
    pool: &PgPool,
    run_id: i64,
) -> Result<Vec<CollectionRunBrandRow>, DbError> {
    let rows = sqlx::query_as::<_, CollectionRunBrandRow>(
        "SELECT id, collection_run_id, brand_id, status, records_processed, \
                error_message, created_at \
         FROM collection_run_brands \
         WHERE collection_run_id = $1",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
