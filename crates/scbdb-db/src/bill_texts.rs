//! Database operations for the `bill_texts` table.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::DbError;

/// A row from the `bill_texts` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BillTextRow {
    pub id: i64,
    pub bill_id: i64,
    pub legiscan_text_id: i64,
    pub text_date: Option<NaiveDate>,
    pub text_type: String,
    pub mime: String,
    pub legiscan_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Inserts a bill text entry, silently skipping duplicates.
///
/// Deduplication is by `legiscan_text_id` (UNIQUE). Text entries from `LegiScan`
/// are immutable once issued, so `ON CONFLICT DO NOTHING` is the full strategy.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn upsert_bill_text(
    pool: &PgPool,
    bill_id: i64,
    legiscan_text_id: i64,
    text_date: Option<NaiveDate>,
    text_type: &str,
    mime: &str,
    legiscan_url: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO bill_texts \
             (bill_id, legiscan_text_id, text_date, text_type, mime, legiscan_url) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (legiscan_text_id) DO NOTHING",
    )
    .bind(bill_id)
    .bind(legiscan_text_id)
    .bind(text_date)
    .bind(text_type)
    .bind(mime)
    .bind(legiscan_url)
    .execute(pool)
    .await?;

    Ok(())
}

/// Returns the stored `legiscan_change_hash` for each known `legiscan_bill_id`
/// in the provided list, in a single query.
///
/// Bills without a stored hash are omitted from the result map. Callers should
/// treat a missing key as "needs fetch".
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn get_bills_stored_hashes(
    pool: &PgPool,
    legiscan_bill_ids: &[i64],
) -> Result<HashMap<i64, String>, DbError> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT legiscan_bill_id, legiscan_change_hash \
         FROM bills \
         WHERE legiscan_bill_id = ANY($1::bigint[]) \
           AND legiscan_change_hash IS NOT NULL",
    )
    .bind(legiscan_bill_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().collect())
}

/// Returns all text entries for a bill identified by its public UUID,
/// ordered by `text_date DESC NULLS LAST`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_bill_texts_by_public_id(
    pool: &PgPool,
    public_id: Uuid,
) -> Result<Vec<BillTextRow>, DbError> {
    let rows = sqlx::query_as::<_, BillTextRow>(
        "SELECT bt.id, bt.bill_id, bt.legiscan_text_id, bt.text_date, bt.text_type, \
                bt.mime, bt.legiscan_url, bt.created_at \
         FROM bill_texts bt \
         JOIN bills b ON b.id = bt.bill_id \
         WHERE b.public_id = $1 AND b.deleted_at IS NULL \
         ORDER BY bt.text_date DESC NULLS LAST",
    )
    .bind(public_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
