//! Database operations for the `brand_media_appearances` table.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::DbError;

/// A row from the `brand_media_appearances` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandMediaAppearanceRow {
    pub id: i64,
    pub brand_id: i64,
    pub brand_signal_id: Option<i64>,
    pub appearance_type: String,
    pub outlet_name: String,
    pub title: Option<String>,
    pub host_or_author: Option<String>,
    pub aired_at: Option<NaiveDate>,
    pub duration_seconds: Option<i32>,
    pub source_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields required to insert a new `brand_media_appearances` row.
#[derive(Debug)]
pub struct NewBrandMediaAppearance<'a> {
    pub brand_id: i64,
    pub brand_signal_id: Option<i64>,
    pub appearance_type: &'a str,
    pub outlet_name: &'a str,
    pub title: Option<&'a str>,
    pub host_or_author: Option<&'a str>,
    pub aired_at: Option<NaiveDate>,
    pub duration_seconds: Option<i32>,
    pub source_url: Option<&'a str>,
    pub notes: Option<&'a str>,
}

/// List all media appearances for a brand, most recent first.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brand_media_appearances(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<BrandMediaAppearanceRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandMediaAppearanceRow>(
        "SELECT id, brand_id, brand_signal_id, appearance_type, outlet_name, \
                title, host_or_author, aired_at, duration_seconds, source_url, \
                notes, created_at, updated_at \
         FROM brand_media_appearances \
         WHERE brand_id = $1 \
         ORDER BY aired_at DESC NULLS LAST, id DESC",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Insert a media appearance. Returns the generated row `id`.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn insert_brand_media_appearance(
    pool: &PgPool,
    appearance: &NewBrandMediaAppearance<'_>,
) -> Result<i64, DbError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO brand_media_appearances \
           (brand_id, brand_signal_id, appearance_type, outlet_name, title, \
            host_or_author, aired_at, duration_seconds, source_url, notes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING id",
    )
    .bind(appearance.brand_id)
    .bind(appearance.brand_signal_id)
    .bind(appearance.appearance_type)
    .bind(appearance.outlet_name)
    .bind(appearance.title)
    .bind(appearance.host_or_author)
    .bind(appearance.aired_at)
    .bind(appearance.duration_seconds)
    .bind(appearance.source_url)
    .bind(appearance.notes)
    .fetch_one(pool)
    .await?;
    Ok(id)
}
