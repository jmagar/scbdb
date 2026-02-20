//! Database operations for the `sentiment_snapshots` table.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgPool;

use crate::DbError;

// ---------------------------------------------------------------------------
// Row type
// ---------------------------------------------------------------------------

/// A row from the `sentiment_snapshots` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SentimentSnapshotRow {
    pub id: i64,
    pub brand_id: i64,
    pub captured_at: DateTime<Utc>,
    pub score: Decimal,
    pub signal_count: i32,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Insert a new sentiment snapshot and return its generated id.
///
/// `score` is a [`Decimal`] bound directly to the `NUMERIC(6,3)` column —
/// values should be in the range [-1.000, 1.000].
/// `metadata` is stored as JSONB and must be a JSON object.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the insert fails.
pub async fn insert_sentiment_snapshot(
    pool: &PgPool,
    brand_id: i64,
    captured_at: DateTime<Utc>,
    score: Decimal,
    signal_count: i32,
    metadata: Value,
) -> Result<i64, DbError> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO sentiment_snapshots \
             (brand_id, captured_at, score, signal_count, metadata) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id",
    )
    .bind(brand_id)
    .bind(captured_at)
    .bind(score)
    .bind(signal_count)
    .bind(metadata)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

/// List recent sentiment snapshots, optionally filtered by brand.
///
/// Results are ordered by `captured_at DESC` then `id DESC`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_sentiment_snapshots(
    pool: &PgPool,
    brand_id: Option<i64>,
    limit: i64,
) -> Result<Vec<SentimentSnapshotRow>, DbError> {
    let rows = match brand_id {
        Some(id) => {
            sqlx::query_as::<_, SentimentSnapshotRow>(
                "SELECT id, brand_id, captured_at, score, signal_count, metadata, created_at \
                 FROM sentiment_snapshots \
                 WHERE brand_id = $1 \
                 ORDER BY captured_at DESC, id DESC \
                 LIMIT $2",
            )
            .bind(id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, SentimentSnapshotRow>(
                "SELECT id, brand_id, captured_at, score, signal_count, metadata, created_at \
                 FROM sentiment_snapshots \
                 ORDER BY captured_at DESC, id DESC \
                 LIMIT $1",
            )
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows)
}

/// Return the most recent sentiment snapshot for a brand, or `None` if none exists.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn get_latest_sentiment_by_brand(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Option<SentimentSnapshotRow>, DbError> {
    let row = sqlx::query_as::<_, SentimentSnapshotRow>(
        "SELECT id, brand_id, captured_at, score, signal_count, metadata, created_at \
         FROM sentiment_snapshots \
         WHERE brand_id = $1 \
         ORDER BY captured_at DESC, id DESC \
         LIMIT 1",
    )
    .bind(brand_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
