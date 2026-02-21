//! Database operations for the `brand_sponsorships` table.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::DbError;

/// A row from the `brand_sponsorships` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandSponsorshipRow {
    pub id: i64,
    pub brand_id: i64,
    pub entity_name: String,
    pub entity_type: String,
    pub deal_type: String,
    pub announced_at: Option<NaiveDate>,
    pub ends_at: Option<NaiveDate>,
    pub source_url: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields required to insert a new `brand_sponsorships` row.
#[derive(Debug)]
pub struct NewBrandSponsorship<'a> {
    pub brand_id: i64,
    pub entity_name: &'a str,
    pub entity_type: &'a str,
    pub deal_type: &'a str,
    pub announced_at: Option<NaiveDate>,
    pub ends_at: Option<NaiveDate>,
    pub source_url: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub is_active: bool,
}

/// List all sponsorships for a brand, ordered by active status then
/// announcement date (most recent first).
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brand_sponsorships(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<BrandSponsorshipRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandSponsorshipRow>(
        "SELECT id, brand_id, entity_name, entity_type, deal_type, \
                announced_at, ends_at, source_url, notes, is_active, \
                created_at, updated_at \
         FROM brand_sponsorships \
         WHERE brand_id = $1 \
         ORDER BY is_active DESC, announced_at DESC NULLS LAST, id DESC",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Insert a sponsorship. Returns the generated row `id`.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn insert_brand_sponsorship(
    pool: &PgPool,
    sponsorship: &NewBrandSponsorship<'_>,
) -> Result<i64, DbError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO brand_sponsorships \
           (brand_id, entity_name, entity_type, deal_type, announced_at, \
            ends_at, source_url, notes, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING id",
    )
    .bind(sponsorship.brand_id)
    .bind(sponsorship.entity_name)
    .bind(sponsorship.entity_type)
    .bind(sponsorship.deal_type)
    .bind(sponsorship.announced_at)
    .bind(sponsorship.ends_at)
    .bind(sponsorship.source_url)
    .bind(sponsorship.notes)
    .bind(sponsorship.is_active)
    .fetch_one(pool)
    .await?;
    Ok(id)
}
