//! Database operations for the `bill_events` table.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::DbError;

/// A row from the `bill_events` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BillEventRow {
    pub id: i64,
    pub bill_id: i64,
    pub event_date: Option<NaiveDate>,
    pub event_type: Option<String>,
    pub chamber: Option<String>,
    pub description: String,
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Inserts a bill event, silently skipping duplicates.
///
/// Deduplication is based on `(bill_id, description, event_date)` via the
/// `idx_bill_events_dedup` unique index (created with `NULLS NOT DISTINCT`
/// so rows with a `NULL` `event_date` are treated as equal). The `ON CONFLICT DO
/// NOTHING` form is atomic; the previous `WHERE NOT EXISTS` form was not.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn upsert_bill_event(
    pool: &PgPool,
    bill_id: i64,
    event_date: Option<NaiveDate>,
    event_type: Option<&str>,
    chamber: Option<&str>,
    description: &str,
    source_url: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO bill_events (bill_id, event_date, event_type, chamber, description, source_url) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (bill_id, description, event_date) DO NOTHING",
    )
    .bind(bill_id)
    .bind(event_date)
    .bind(event_type)
    .bind(chamber)
    .bind(description)
    .bind(source_url)
    .execute(pool)
    .await?;

    Ok(())
}

/// Returns all events for a given bill, ordered by `event_date DESC NULLS LAST`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_bill_events(pool: &PgPool, bill_id: i64) -> Result<Vec<BillEventRow>, DbError> {
    let rows = sqlx::query_as::<_, BillEventRow>(
        "SELECT id, bill_id, event_date, event_type, chamber, description, source_url, created_at \
         FROM bill_events \
         WHERE bill_id = $1 \
         ORDER BY event_date DESC NULLS LAST",
    )
    .bind(bill_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Returns events for multiple bills in a single query, grouped by `bill_id`.
///
/// Each bill's events are ordered by `event_date DESC NULLS LAST`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_bill_events_batch(
    pool: &PgPool,
    bill_ids: &[i64],
) -> Result<std::collections::HashMap<i64, Vec<BillEventRow>>, DbError> {
    let rows = sqlx::query_as::<_, BillEventRow>(
        "SELECT id, bill_id, event_date, event_type, chamber, description, source_url, created_at \
         FROM bill_events \
         WHERE bill_id = ANY($1::bigint[]) \
         ORDER BY bill_id, event_date DESC NULLS LAST",
    )
    .bind(bill_ids)
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<i64, Vec<BillEventRow>> =
        std::collections::HashMap::new();
    for row in rows {
        map.entry(row.bill_id).or_default().push(row);
    }
    Ok(map)
}

/// Returns all events for a bill identified by its public UUID,
/// ordered by `event_date DESC NULLS LAST`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_bill_events_by_public_id(
    pool: &PgPool,
    public_id: Uuid,
) -> Result<Vec<BillEventRow>, DbError> {
    let rows = sqlx::query_as::<_, BillEventRow>(
        "SELECT be.id, be.bill_id, be.event_date, be.event_type, be.chamber, be.description, \
                be.source_url, be.created_at \
         FROM bill_events be \
         JOIN bills b ON b.id = be.bill_id \
         WHERE b.public_id = $1 AND b.deleted_at IS NULL \
         ORDER BY be.event_date DESC NULLS LAST",
    )
    .bind(public_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
