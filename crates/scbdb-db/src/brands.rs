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
    pub twitter_handle: Option<String>,
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
                store_locator_url, notes, twitter_handle, is_active, created_at, updated_at, deleted_at \
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
                store_locator_url, notes, twitter_handle, is_active, created_at, updated_at, deleted_at \
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
                store_locator_url, notes, twitter_handle, is_active, created_at, updated_at, deleted_at \
         FROM brands \
         WHERE store_locator_url IS NOT NULL AND is_active = true AND deleted_at IS NULL \
         ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Creates a new brand row and returns the full inserted row.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails (including unique constraint violations).
#[allow(clippy::too_many_arguments)] // public API for full brand creation; no sensible grouping
pub async fn create_brand(
    pool: &PgPool,
    name: &str,
    slug: &str,
    relationship: &str,
    tier: i16,
    domain: Option<&str>,
    shop_url: Option<&str>,
    store_locator_url: Option<&str>,
    twitter_handle: Option<&str>,
    notes: Option<&str>,
) -> Result<BrandRow, DbError> {
    let row = sqlx::query_as::<_, BrandRow>(
        "INSERT INTO brands \
           (name, slug, relationship, tier, domain, shop_url, store_locator_url, \
            twitter_handle, notes, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true) \
         RETURNING id, public_id, name, slug, relationship, tier, domain, shop_url, logo_url, \
                   store_locator_url, notes, twitter_handle, is_active, created_at, updated_at, deleted_at",
    )
    .bind(name)
    .bind(slug)
    .bind(relationship)
    .bind(tier)
    .bind(domain)
    .bind(shop_url)
    .bind(store_locator_url)
    .bind(twitter_handle)
    .bind(notes)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Updates core metadata fields for an existing brand.
///
/// All `Option` fields are overlaid onto the existing row: `Some(v)` sets the value,
/// `None` preserves the existing value.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
#[allow(clippy::too_many_arguments)] // public API for partial brand update; no sensible grouping
pub async fn update_brand(
    pool: &PgPool,
    brand_id: i64,
    name: Option<&str>,
    relationship: Option<&str>,
    tier: Option<i16>,
    domain: Option<Option<&str>>,
    shop_url: Option<Option<&str>>,
    store_locator_url: Option<Option<&str>>,
    twitter_handle: Option<Option<&str>>,
    notes: Option<Option<&str>>,
) -> Result<BrandRow, DbError> {
    // Load existing row first, then overlay supplied values.
    let existing = sqlx::query_as::<_, BrandRow>(
        "SELECT id, public_id, name, slug, relationship, tier, domain, shop_url, logo_url, \
                store_locator_url, notes, twitter_handle, is_active, created_at, updated_at, deleted_at \
         FROM brands WHERE id = $1",
    )
    .bind(brand_id)
    .fetch_one(pool)
    .await?;

    let new_name = name.unwrap_or(&existing.name).to_owned();
    let new_relationship = relationship.unwrap_or(&existing.relationship).to_owned();
    let new_tier = tier.unwrap_or(existing.tier);
    let new_domain = domain.unwrap_or(existing.domain.as_deref());
    let new_shop_url = shop_url.unwrap_or(existing.shop_url.as_deref());
    let new_store_locator_url = store_locator_url.unwrap_or(existing.store_locator_url.as_deref());
    let new_twitter_handle = twitter_handle.unwrap_or(existing.twitter_handle.as_deref());
    let new_notes = notes.unwrap_or(existing.notes.as_deref());

    let row = sqlx::query_as::<_, BrandRow>(
        "UPDATE brands \
         SET name = $2, relationship = $3, tier = $4, domain = $5, shop_url = $6, \
             store_locator_url = $7, twitter_handle = $8, notes = $9, updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, public_id, name, slug, relationship, tier, domain, shop_url, logo_url, \
                   store_locator_url, notes, twitter_handle, is_active, created_at, updated_at, deleted_at",
    )
    .bind(brand_id)
    .bind(&new_name)
    .bind(&new_relationship)
    .bind(new_tier)
    .bind(new_domain)
    .bind(new_shop_url)
    .bind(new_store_locator_url)
    .bind(new_twitter_handle)
    .bind(new_notes)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Soft-deletes a brand by setting `is_active = false` and `deleted_at = NOW()`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn deactivate_brand(pool: &PgPool, brand_id: i64) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE brands \
         SET is_active = false, deleted_at = NOW(), updated_at = NOW() \
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(brand_id)
    .execute(pool)
    .await?;
    Ok(())
}
