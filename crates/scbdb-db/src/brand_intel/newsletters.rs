//! Database operations for the `brand_newsletters` table.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::DbError;

/// A row from the `brand_newsletters` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandNewsletterRow {
    pub id: i64,
    pub brand_id: i64,
    pub list_name: String,
    pub subscribe_url: Option<String>,
    pub unsubscribe_url: Option<String>,
    pub inbox_address: Option<String>,
    pub subscribed_at: Option<DateTime<Utc>>,
    pub last_received_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields required to insert a new `brand_newsletters` row.
#[derive(Debug)]
pub struct NewBrandNewsletter<'a> {
    pub brand_id: i64,
    pub list_name: &'a str,
    pub subscribe_url: Option<&'a str>,
    pub unsubscribe_url: Option<&'a str>,
    pub inbox_address: Option<&'a str>,
    pub subscribed_at: Option<DateTime<Utc>>,
    pub last_received_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub notes: Option<&'a str>,
}

/// List all newsletters for a brand, ordered by active status then list name.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brand_newsletters(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<BrandNewsletterRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandNewsletterRow>(
        "SELECT id, brand_id, list_name, subscribe_url, unsubscribe_url, \
                inbox_address, subscribed_at, last_received_at, is_active, \
                notes, created_at, updated_at \
         FROM brand_newsletters \
         WHERE brand_id = $1 \
         ORDER BY is_active DESC, list_name ASC, id DESC",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Insert a newsletter. Returns the generated row `id`.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn insert_brand_newsletter(
    pool: &PgPool,
    newsletter: &NewBrandNewsletter<'_>,
) -> Result<i64, DbError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO brand_newsletters \
           (brand_id, list_name, subscribe_url, unsubscribe_url, inbox_address, \
            subscribed_at, last_received_at, is_active, notes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING id",
    )
    .bind(newsletter.brand_id)
    .bind(newsletter.list_name)
    .bind(newsletter.subscribe_url)
    .bind(newsletter.unsubscribe_url)
    .bind(newsletter.inbox_address)
    .bind(newsletter.subscribed_at)
    .bind(newsletter.last_received_at)
    .bind(newsletter.is_active)
    .bind(newsletter.notes)
    .fetch_one(pool)
    .await?;
    Ok(id)
}
