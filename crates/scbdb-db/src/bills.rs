//! Database operations for `bills` and `bill_events`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::DbError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

/// A row from the `bills` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BillRow {
    pub id: i64,
    pub public_id: Uuid,
    pub jurisdiction: String,
    pub session: Option<String>,
    pub bill_number: String,
    pub title: String,
    pub summary: Option<String>,
    pub status: String,
    pub status_date: Option<NaiveDate>,
    pub introduced_date: Option<NaiveDate>,
    pub last_action_date: Option<NaiveDate>,
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

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

// ---------------------------------------------------------------------------
// bills operations
// ---------------------------------------------------------------------------

/// Upserts a bill row.
///
/// Conflicts on `(jurisdiction, bill_number)` update mutable fields but
/// preserve the original `introduced_date` (set once on first insert).
///
/// Returns the internal `id` of the upserted row.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the upsert fails.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_bill(
    pool: &PgPool,
    jurisdiction: &str,
    bill_number: &str,
    title: &str,
    summary: Option<&str>,
    status: &str,
    status_date: Option<NaiveDate>,
    introduced_date: Option<NaiveDate>,
    last_action_date: Option<NaiveDate>,
    session: Option<&str>,
    source_url: Option<&str>,
) -> Result<i64, DbError> {
    let public_id = Uuid::new_v4();

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO bills \
             (public_id, jurisdiction, session, bill_number, title, summary, \
              status, status_date, introduced_date, last_action_date, source_url) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT (jurisdiction, bill_number) DO UPDATE SET \
             session          = EXCLUDED.session, \
             title            = EXCLUDED.title, \
             summary          = EXCLUDED.summary, \
             status           = EXCLUDED.status, \
             status_date      = EXCLUDED.status_date, \
             last_action_date = EXCLUDED.last_action_date, \
             source_url       = EXCLUDED.source_url, \
             updated_at       = NOW() \
         RETURNING id",
    )
    .bind(public_id)
    .bind(jurisdiction)
    .bind(session)
    .bind(bill_number)
    .bind(title)
    .bind(summary)
    .bind(status)
    .bind(status_date)
    .bind(introduced_date)
    .bind(last_action_date)
    .bind(source_url)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// bill_events operations
// ---------------------------------------------------------------------------

/// Inserts a bill event, silently skipping duplicates.
///
/// Deduplication is based on `(bill_id, description, event_date)` via the
/// `idx_bill_events_dedup` unique index (created with `NULLS NOT DISTINCT`
/// so rows with a NULL event_date are treated as equal). The `ON CONFLICT DO
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

// ---------------------------------------------------------------------------
// Query operations
// ---------------------------------------------------------------------------

/// Returns non-deleted bills, optionally filtered by jurisdiction.
///
/// Results are ordered by `last_action_date DESC NULLS LAST`, then
/// `status_date DESC NULLS LAST`, limited to `limit` rows.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_bills(
    pool: &PgPool,
    jurisdiction: Option<&str>,
    limit: i64,
) -> Result<Vec<BillRow>, DbError> {
    let rows = sqlx::query_as::<_, BillRow>(
        "SELECT id, public_id, jurisdiction, session, bill_number, title, summary, \
                status, status_date, introduced_date, last_action_date, source_url, \
                created_at, updated_at, deleted_at \
         FROM bills \
         WHERE deleted_at IS NULL \
           AND ($1::TEXT IS NULL OR jurisdiction = $1) \
         ORDER BY last_action_date DESC NULLS LAST, status_date DESC NULLS LAST \
         LIMIT $2",
    )
    .bind(jurisdiction)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Returns a single non-deleted bill by jurisdiction and bill number, or
/// `None` if not found.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn get_bill_by_jurisdiction_number(
    pool: &PgPool,
    jurisdiction: &str,
    bill_number: &str,
) -> Result<Option<BillRow>, DbError> {
    let row = sqlx::query_as::<_, BillRow>(
        "SELECT id, public_id, jurisdiction, session, bill_number, title, summary, \
                status, status_date, introduced_date, last_action_date, source_url, \
                created_at, updated_at, deleted_at \
         FROM bills \
         WHERE jurisdiction = $1 AND bill_number = $2 AND deleted_at IS NULL",
    )
    .bind(jurisdiction)
    .bind(bill_number)
    .fetch_optional(pool)
    .await?;

    Ok(row)
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
