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
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Returns all active brands (`is_active = true`), ordered by name.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_active_brands(pool: &PgPool) -> Result<Vec<BrandRow>, DbError> {
    let rows = sqlx::query_as::<_, BrandRow>(
        "SELECT id, public_id, name, slug, relationship, tier, domain, shop_url, notes, \
                is_active, created_at, updated_at, deleted_at \
         FROM brands \
         WHERE is_active = true \
         ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Returns a single brand by slug, or `None` if not found.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn get_brand_by_slug(pool: &PgPool, slug: &str) -> Result<Option<BrandRow>, DbError> {
    let row = sqlx::query_as::<_, BrandRow>(
        "SELECT id, public_id, name, slug, relationship, tier, domain, shop_url, notes, \
                is_active, created_at, updated_at, deleted_at \
         FROM brands \
         WHERE slug = $1",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
