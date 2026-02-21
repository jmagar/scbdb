//! Database operations for the `brand_funding_events` table.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::DbError;

/// A row from the `brand_funding_events` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandFundingEventRow {
    pub id: i64,
    pub brand_id: i64,
    pub event_type: String,
    pub amount_usd: Option<i64>,
    pub announced_at: Option<NaiveDate>,
    pub investors: Option<Vec<String>>,
    pub acquirer: Option<String>,
    pub source_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Fields required to insert a new `brand_funding_events` row.
#[derive(Debug)]
pub struct NewBrandFundingEvent<'a> {
    pub brand_id: i64,
    pub event_type: &'a str,
    pub amount_usd: Option<i64>,
    pub announced_at: Option<NaiveDate>,
    pub investors: Option<&'a [String]>,
    pub acquirer: Option<&'a str>,
    pub source_url: Option<&'a str>,
    pub notes: Option<&'a str>,
}

/// List all funding events for a brand, most recent first.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brand_funding_events(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<BrandFundingEventRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandFundingEventRow>(
        "SELECT id, brand_id, event_type, amount_usd, announced_at, \
                investors, acquirer, source_url, notes, created_at \
         FROM brand_funding_events \
         WHERE brand_id = $1 \
         ORDER BY announced_at DESC NULLS LAST, id DESC",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Insert a funding event. Returns the generated row `id`.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn insert_brand_funding_event(
    pool: &PgPool,
    event: &NewBrandFundingEvent<'_>,
) -> Result<i64, DbError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO brand_funding_events \
           (brand_id, event_type, amount_usd, announced_at, investors, \
            acquirer, source_url, notes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING id",
    )
    .bind(event.brand_id)
    .bind(event.event_type)
    .bind(event.amount_usd)
    .bind(event.announced_at)
    .bind(event.investors)
    .bind(event.acquirer)
    .bind(event.source_url)
    .bind(event.notes)
    .fetch_one(pool)
    .await?;
    Ok(id)
}
