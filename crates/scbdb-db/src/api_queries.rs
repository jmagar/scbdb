//! Read-model queries used by `scbdb-server` dashboard endpoints.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::DbError;

/// Product list row tailored for API/dashboard views.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProductDashboardRow {
    pub product_id: i64,
    pub product_name: String,
    pub product_status: Option<String>,
    pub vendor: Option<String>,
    pub source_url: Option<String>,
    pub primary_image_url: Option<String>,
    pub brand_name: String,
    pub brand_slug: String,
    pub brand_logo_url: Option<String>,
    pub relationship: String,
    pub tier: i16,
    pub variant_count: i64,
    pub latest_price: Option<Decimal>,
    pub latest_price_captured_at: Option<DateTime<Utc>>,
}

/// Input filters for product listing.
#[derive(Debug, Clone, Default)]
pub struct ProductListFilters<'a> {
    pub brand_slug: Option<&'a str>,
    pub relationship: Option<&'a str>,
    pub tier: Option<i16>,
    pub limit: i64,
}

/// Price snapshot row for API/dashboard views.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PriceSnapshotDashboardRow {
    pub captured_at: DateTime<Utc>,
    pub currency_code: String,
    pub price: Decimal,
    pub compare_at_price: Option<Decimal>,
    pub variant_title: Option<String>,
    pub source_variant_id: String,
    pub product_name: String,
    pub brand_name: String,
    pub brand_slug: String,
    pub brand_logo_url: Option<String>,
}

/// Input filters for price snapshot listing.
#[derive(Debug, Clone, Default)]
pub struct PriceSnapshotFilters<'a> {
    pub brand_slug: Option<&'a str>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: i64,
}

/// Aggregated pricing metrics per brand.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PricingSummaryRow {
    pub brand_name: String,
    pub brand_slug: String,
    pub brand_logo_url: Option<String>,
    pub variant_count: i64,
    pub avg_price: Decimal,
    pub min_price: Decimal,
    pub max_price: Decimal,
    pub latest_capture_at: DateTime<Utc>,
}

/// Returns product cards with brand context and latest observed price.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_products_dashboard(
    pool: &PgPool,
    filters: ProductListFilters<'_>,
) -> Result<Vec<ProductDashboardRow>, DbError> {
    let rows = sqlx::query_as::<_, ProductDashboardRow>(
        "SELECT \
             p.id AS product_id, \
             p.name AS product_name, \
             p.status AS product_status, \
             p.vendor, \
             p.source_url, \
             p.metadata->>'primary_image_url' AS primary_image_url, \
             b.name AS brand_name, \
             b.slug AS brand_slug, \
             b.logo_url AS brand_logo_url, \
             b.relationship, \
             b.tier, \
             COUNT(v.id)::bigint AS variant_count, \
             latest.price AS latest_price, \
             latest.captured_at AS latest_price_captured_at \
         FROM products p \
         JOIN brands b ON b.id = p.brand_id \
         LEFT JOIN product_variants v ON v.product_id = p.id \
         LEFT JOIN LATERAL ( \
             SELECT ps.price, ps.captured_at \
             FROM product_variants pv \
             JOIN price_snapshots ps ON ps.variant_id = pv.id \
             WHERE pv.product_id = p.id \
             ORDER BY ps.captured_at DESC, ps.id DESC \
             LIMIT 1 \
         ) latest ON TRUE \
         WHERE p.deleted_at IS NULL \
           AND b.deleted_at IS NULL \
           AND ($1::TEXT IS NULL OR b.slug = $1) \
           AND ($2::TEXT IS NULL OR b.relationship = $2) \
           AND ($3::SMALLINT IS NULL OR b.tier = $3) \
         GROUP BY p.id, p.name, p.status, p.vendor, p.source_url, p.metadata, b.name, b.slug, \
                  b.logo_url, b.relationship, b.tier, latest.price, latest.captured_at \
         ORDER BY p.updated_at DESC \
         LIMIT $4",
    )
    .bind(filters.brand_slug)
    .bind(filters.relationship)
    .bind(filters.tier)
    .bind(filters.limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Returns pricing snapshots with product/brand context for dashboard displays.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_price_snapshots_dashboard(
    pool: &PgPool,
    filters: PriceSnapshotFilters<'_>,
) -> Result<Vec<PriceSnapshotDashboardRow>, DbError> {
    let rows = sqlx::query_as::<_, PriceSnapshotDashboardRow>(
        "SELECT \
             ps.captured_at, \
             ps.currency_code::text AS currency_code, \
             ps.price, \
             ps.compare_at_price, \
             pv.title AS variant_title, \
             pv.source_variant_id, \
             p.name AS product_name, \
             b.name AS brand_name, \
             b.slug AS brand_slug, \
             b.logo_url AS brand_logo_url \
         FROM price_snapshots ps \
         JOIN product_variants pv ON pv.id = ps.variant_id \
         JOIN products p ON p.id = pv.product_id \
         JOIN brands b ON b.id = p.brand_id \
         WHERE p.deleted_at IS NULL \
           AND b.deleted_at IS NULL \
           AND ($1::TEXT IS NULL OR b.slug = $1) \
           AND ($2::timestamptz IS NULL OR ps.captured_at >= $2) \
           AND ($3::timestamptz IS NULL OR ps.captured_at <= $3) \
         ORDER BY ps.captured_at DESC, ps.id DESC \
         LIMIT $4",
    )
    .bind(filters.brand_slug)
    .bind(filters.from)
    .bind(filters.to)
    .bind(filters.limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Returns latest-price summary metrics grouped by brand.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_pricing_summary(pool: &PgPool) -> Result<Vec<PricingSummaryRow>, DbError> {
    let rows = sqlx::query_as::<_, PricingSummaryRow>(
        "WITH latest_variant_prices AS ( \
             SELECT DISTINCT ON (ps.variant_id) \
                 pv.product_id, \
                 ps.variant_id, \
                 ps.price, \
                 ps.captured_at \
             FROM price_snapshots ps \
             JOIN product_variants pv ON pv.id = ps.variant_id \
             ORDER BY ps.variant_id, ps.captured_at DESC, ps.id DESC \
         ) \
         SELECT \
             b.name AS brand_name, \
             b.slug AS brand_slug, \
             b.logo_url AS brand_logo_url, \
             COUNT(lvp.variant_id)::bigint AS variant_count, \
             AVG(lvp.price)::numeric(10,2) AS avg_price, \
             MIN(lvp.price) AS min_price, \
             MAX(lvp.price) AS max_price, \
             MAX(lvp.captured_at) AS latest_capture_at \
         FROM latest_variant_prices lvp \
         JOIN products p ON p.id = lvp.product_id \
         JOIN brands b ON b.id = p.brand_id \
         WHERE p.deleted_at IS NULL \
           AND b.deleted_at IS NULL \
         GROUP BY b.name, b.slug, b.logo_url \
         ORDER BY b.name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Aggregated sentiment: most recent snapshot per brand.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SentimentSummaryRow {
    pub brand_name: String,
    pub brand_slug: String,
    pub score: Decimal,
    pub signal_count: i32,
    pub captured_at: DateTime<Utc>,
}

/// Recent sentiment snapshot with brand context.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SentimentSnapshotDashboardRow {
    pub brand_name: String,
    pub brand_slug: String,
    pub score: Decimal,
    pub signal_count: i32,
    pub captured_at: DateTime<Utc>,
}
