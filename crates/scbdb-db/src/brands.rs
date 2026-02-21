//! Database operations for the `brands` table.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::DbError;

// ---------------------------------------------------------------------------
// Row type
// ---------------------------------------------------------------------------

/// A row from the `brands` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandRow {
    pub id: i64,
    pub public_id: Uuid,
    pub name: String,
    pub slug: String,
    pub relationship: String,
    pub tier: i16,
    pub domain: Option<String>,
    pub shop_url: Option<String>,
    pub logo_url: Option<String>,
    pub store_locator_url: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Returns all active, non-deleted brands, ordered by name.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_active_brands(pool: &PgPool) -> Result<Vec<BrandRow>, DbError> {
    let rows = sqlx::query_as::<_, BrandRow>(
        "SELECT id, public_id, name, slug, relationship, tier, domain, shop_url, logo_url, \
                store_locator_url, notes, is_active, created_at, updated_at, deleted_at \
         FROM brands \
         WHERE is_active = true AND deleted_at IS NULL \
         ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Returns a single active, non-deleted brand by slug, or `None` if not found.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn get_brand_by_slug(pool: &PgPool, slug: &str) -> Result<Option<BrandRow>, DbError> {
    let row = sqlx::query_as::<_, BrandRow>(
        "SELECT id, public_id, name, slug, relationship, tier, domain, shop_url, logo_url, \
                store_locator_url, notes, is_active, created_at, updated_at, deleted_at \
         FROM brands \
         WHERE slug = $1 AND is_active = true AND deleted_at IS NULL",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Updates `brands.logo_url` for a given brand id.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn update_brand_logo(
    pool: &PgPool,
    brand_id: i64,
    logo_url: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE brands \
         SET logo_url = $1, updated_at = NOW() \
         WHERE id = $2",
    )
    .bind(logo_url)
    .bind(brand_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Updates `brands.store_locator_url` for a given brand id.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn update_brand_store_locator_url(
    pool: &PgPool,
    brand_id: i64,
    url: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE brands \
         SET store_locator_url = $1, updated_at = NOW() \
         WHERE id = $2",
    )
    .bind(url)
    .bind(brand_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns all active, non-deleted brands that have a store locator URL, ordered by name.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_brands_with_locator(pool: &PgPool) -> Result<Vec<BrandRow>, DbError> {
    let rows = sqlx::query_as::<_, BrandRow>(
        "SELECT id, public_id, name, slug, relationship, tier, domain, shop_url, logo_url, \
                store_locator_url, notes, is_active, created_at, updated_at, deleted_at \
         FROM brands \
         WHERE store_locator_url IS NOT NULL AND is_active = true AND deleted_at IS NULL \
         ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
