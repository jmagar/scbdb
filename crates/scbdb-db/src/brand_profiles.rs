//! Database operations for `brand_profiles` and `brand_social_handles` tables.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::DbError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

/// A row from the `brand_profiles` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandProfileRow {
    pub id: i64,
    pub brand_id: i64,
    pub tagline: Option<String>,
    pub description: Option<String>,
    pub founded_year: Option<i16>,
    pub hq_city: Option<String>,
    pub hq_state: Option<String>,
    pub hq_country: String,
    pub parent_company: Option<String>,
    pub parent_domain: Option<String>,
    pub ceo_name: Option<String>,
    pub employee_count_approx: Option<i32>,
    pub total_funding_usd: Option<i64>,
    pub latest_valuation_usd: Option<i64>,
    pub funding_stage: Option<String>,
    pub stock_ticker: Option<String>,
    pub stock_exchange: Option<String>,
    pub hero_image_url: Option<String>,
    pub profile_completed_at: Option<DateTime<Utc>>,
    pub last_enriched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A row from the `brand_social_handles` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandSocialHandleRow {
    pub id: i64,
    pub brand_id: i64,
    pub platform: String,
    pub handle: String,
    pub profile_url: Option<String>,
    pub follower_count: Option<i32>,
    pub is_verified: Option<bool>,
    pub is_active: bool,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Get the brand profile for a given `brand_id`, if it exists.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn get_brand_profile(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Option<BrandProfileRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandProfileRow>(
        "SELECT id, brand_id, tagline, description, founded_year, hq_city, hq_state, \
                hq_country, parent_company, parent_domain, ceo_name, employee_count_approx, \
                total_funding_usd, latest_valuation_usd, funding_stage, stock_ticker, \
                stock_exchange, hero_image_url, profile_completed_at, last_enriched_at, \
                created_at, updated_at \
         FROM brand_profiles WHERE brand_id = $1",
    )
    .bind(brand_id)
    .fetch_optional(pool)
    .await?)
}

/// Upsert (insert or update) a brand profile row.
///
/// Uses `COALESCE` to preserve existing non-null values when updating, so
/// callers can supply partial data without overwriting already-enriched fields.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_brand_profile(
    pool: &PgPool,
    brand_id: i64,
    tagline: Option<&str>,
    description: Option<&str>,
    founded_year: Option<i16>,
    hq_city: Option<&str>,
    hq_state: Option<&str>,
    parent_company: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO brand_profiles \
           (brand_id, tagline, description, founded_year, hq_city, hq_state, parent_company, last_enriched_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW()) \
         ON CONFLICT (brand_id) DO UPDATE SET \
           tagline          = COALESCE(EXCLUDED.tagline,          brand_profiles.tagline), \
           description      = COALESCE(EXCLUDED.description,      brand_profiles.description), \
           founded_year     = COALESCE(EXCLUDED.founded_year,     brand_profiles.founded_year), \
           hq_city          = COALESCE(EXCLUDED.hq_city,          brand_profiles.hq_city), \
           hq_state         = COALESCE(EXCLUDED.hq_state,         brand_profiles.hq_state), \
           parent_company   = COALESCE(EXCLUDED.parent_company,   brand_profiles.parent_company), \
           last_enriched_at = NOW(), \
           updated_at       = NOW()",
    )
    .bind(brand_id)
    .bind(tagline)
    .bind(description)
    .bind(founded_year)
    .bind(hq_city)
    .bind(hq_state)
    .bind(parent_company)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns IDs of active, non-deleted brands that have no `brand_profiles` row.
///
/// Used by the scheduler to detect brands that need profile intake.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brands_without_profiles(pool: &PgPool) -> Result<Vec<i64>, DbError> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT b.id FROM brands b \
         LEFT JOIN brand_profiles bp ON bp.brand_id = b.id \
         WHERE b.is_active = true AND b.deleted_at IS NULL AND bp.id IS NULL \
         ORDER BY b.id",
    )
    .fetch_all(pool)
    .await?)
}

/// Get all active social handles for a brand, ordered by platform.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brand_social_handles(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<BrandSocialHandleRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandSocialHandleRow>(
        "SELECT id, brand_id, platform, handle, profile_url, follower_count, \
                is_verified, is_active, last_checked_at, created_at, updated_at \
         FROM brand_social_handles \
         WHERE brand_id = $1 AND is_active = true \
         ORDER BY platform",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Overwrite the brand profile row for the given brand.
///
/// Unlike [`upsert_brand_profile`], this does **not** use `COALESCE` — every supplied
/// value (including `None`) is written directly, allowing the operator to clear fields.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
#[allow(clippy::too_many_arguments)]
pub async fn overwrite_brand_profile(
    pool: &PgPool,
    brand_id: i64,
    tagline: Option<&str>,
    description: Option<&str>,
    founded_year: Option<i16>,
    hq_city: Option<&str>,
    hq_state: Option<&str>,
    ceo_name: Option<&str>,
    funding_stage: Option<&str>,
    employee_count_approx: Option<i32>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO brand_profiles \
           (brand_id, tagline, description, founded_year, hq_city, hq_state, \
            ceo_name, funding_stage, employee_count_approx, last_enriched_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW()) \
         ON CONFLICT (brand_id) DO UPDATE SET \
           tagline               = EXCLUDED.tagline, \
           description           = EXCLUDED.description, \
           founded_year          = EXCLUDED.founded_year, \
           hq_city               = EXCLUDED.hq_city, \
           hq_state              = EXCLUDED.hq_state, \
           ceo_name              = EXCLUDED.ceo_name, \
           funding_stage         = EXCLUDED.funding_stage, \
           employee_count_approx = EXCLUDED.employee_count_approx, \
           last_enriched_at      = NOW(), \
           updated_at            = NOW()",
    )
    .bind(brand_id)
    .bind(tagline)
    .bind(description)
    .bind(founded_year)
    .bind(hq_city)
    .bind(hq_state)
    .bind(ceo_name)
    .bind(funding_stage)
    .bind(employee_count_approx)
    .execute(pool)
    .await?;
    Ok(())
}

/// Replace all active social handles for a brand.
///
/// Deactivates all existing handles, then upserts the supplied map.
/// Uses a transaction to keep the operation atomic.
///
/// # Errors
/// Returns [`DbError`] on database query failure.
#[allow(clippy::implicit_hasher)] // public API; callers pass HashMap directly
pub async fn replace_brand_social_handles(
    pool: &PgPool,
    brand_id: i64,
    handles: &std::collections::HashMap<String, String>,
) -> Result<(), DbError> {
    let mut tx = pool.begin().await?;

    // Deactivate all existing handles for this brand.
    sqlx::query(
        "UPDATE brand_social_handles SET is_active = false, updated_at = NOW() \
         WHERE brand_id = $1",
    )
    .bind(brand_id)
    .execute(&mut *tx)
    .await?;

    // Upsert each supplied handle.
    for (platform, handle) in handles {
        sqlx::query(
            "INSERT INTO brand_social_handles (brand_id, platform, handle, is_active) \
             VALUES ($1, $2, $3, true) \
             ON CONFLICT (brand_id, platform, handle) DO UPDATE \
               SET is_active = true, updated_at = NOW()",
        )
        .bind(brand_id)
        .bind(platform)
        .bind(handle)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Replace all active domains for a brand.
///
/// Deactivates all existing domains, then upserts the supplied list.
/// Uses a transaction to keep the operation atomic.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn replace_brand_domains(
    pool: &PgPool,
    brand_id: i64,
    domains: &[String],
) -> Result<(), DbError> {
    let mut tx = pool.begin().await?;

    // Deactivate all existing domains for this brand.
    sqlx::query(
        "UPDATE brand_domains SET is_active = false, updated_at = NOW() \
         WHERE brand_id = $1",
    )
    .bind(brand_id)
    .execute(&mut *tx)
    .await?;

    // Upsert each supplied domain.
    for domain in domains {
        sqlx::query(
            "INSERT INTO brand_domains (brand_id, domain, domain_type, is_active) \
             VALUES ($1, $2, 'primary', true) \
             ON CONFLICT (brand_id, domain) DO UPDATE \
               SET is_active = true, updated_at = NOW()",
        )
        .bind(brand_id)
        .bind(domain)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
