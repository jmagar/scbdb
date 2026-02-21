//! Database operations for the `bills` table.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::DbError;

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
    pub legiscan_bill_id: Option<i64>,
    pub legiscan_change_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Upserts a bill row.
///
/// Conflicts on `(jurisdiction, bill_number)` update mutable fields but
/// preserve the original `introduced_date` (set once on first insert).
/// `legiscan_change_hash` is updated on every upsert so the ingest pipeline
/// can detect future changes via `get_bills_stored_hashes`.
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
    legiscan_bill_id: Option<i64>,
    legiscan_change_hash: Option<&str>,
) -> Result<i64, DbError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO bills \
             (jurisdiction, session, bill_number, title, summary, \
              status, status_date, introduced_date, last_action_date, source_url, \
              legiscan_bill_id, legiscan_change_hash) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         ON CONFLICT (jurisdiction, bill_number) DO UPDATE SET \
             session               = EXCLUDED.session, \
             title                 = EXCLUDED.title, \
             summary               = EXCLUDED.summary, \
             status                = EXCLUDED.status, \
             status_date           = EXCLUDED.status_date, \
             last_action_date      = EXCLUDED.last_action_date, \
             source_url            = EXCLUDED.source_url, \
             legiscan_bill_id      = COALESCE(EXCLUDED.legiscan_bill_id, bills.legiscan_bill_id), \
             legiscan_change_hash  = COALESCE(EXCLUDED.legiscan_change_hash, bills.legiscan_change_hash), \
             updated_at            = NOW() \
         RETURNING id",
    )
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
    .bind(legiscan_bill_id)
    .bind(legiscan_change_hash)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

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
                legiscan_bill_id, legiscan_change_hash, created_at, updated_at, deleted_at \
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
                legiscan_bill_id, legiscan_change_hash, created_at, updated_at, deleted_at \
         FROM bills \
         WHERE jurisdiction = $1 AND bill_number = $2 AND deleted_at IS NULL",
    )
    .bind(jurisdiction)
    .bind(bill_number)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Returns a single non-deleted bill by public UUID, or `None` if not found.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn get_bill_by_public_id(
    pool: &PgPool,
    public_id: Uuid,
) -> Result<Option<BillRow>, DbError> {
    let row = sqlx::query_as::<_, BillRow>(
        "SELECT id, public_id, jurisdiction, session, bill_number, title, summary, \
                status, status_date, introduced_date, last_action_date, source_url, \
                legiscan_bill_id, legiscan_change_hash, created_at, updated_at, deleted_at \
         FROM bills \
         WHERE public_id = $1 AND deleted_at IS NULL",
    )
    .bind(public_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
