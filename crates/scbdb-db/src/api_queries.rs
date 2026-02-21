//! Read-model queries used by `scbdb-server` dashboard endpoints.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
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
///
/// `limit` is `None` to return all products, or `Some(n)` to cap results.
#[derive(Debug, Clone, Default)]
pub struct ProductListFilters<'a> {
    pub brand_slug: Option<&'a str>,
    pub relationship: Option<&'a str>,
    pub tier: Option<i16>,
    pub limit: Option<i64>,
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
             product_id, product_name, product_status, vendor, source_url, \
             primary_image_url, brand_name, brand_slug, brand_logo_url, \
             relationship, tier, variant_count, latest_price, \
             latest_price_captured_at \
         FROM view_products_dashboard \
         WHERE deleted_at IS NULL \
           AND brand_deleted_at IS NULL \
           AND ($1::TEXT IS NULL OR brand_slug = $1) \
           AND ($2::TEXT IS NULL OR relationship = $2) \
           AND ($3::SMALLINT IS NULL OR tier = $3) \
         ORDER BY updated_at DESC \
         LIMIT COALESCE($4, 9223372036854775807)",
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
        "SELECT \
             brand_name, brand_slug, brand_logo_url, variant_count, \
             avg_price, min_price, max_price, latest_capture_at \
         FROM view_pricing_summary \
         WHERE product_deleted_at IS NULL \
           AND brand_deleted_at IS NULL \
         ORDER BY brand_name",
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
    pub metadata: Value,
}

/// Recent sentiment snapshot with brand context.
// TODO: consider merging with `SentimentSummaryRow` if these remain structurally identical.
// They are separate types in case the dashboard row gains extra fields (e.g. brand_logo_url).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SentimentSnapshotDashboardRow {
    pub brand_name: String,
    pub brand_slug: String,
    pub score: Decimal,
    pub signal_count: i32,
    pub captured_at: DateTime<Utc>,
    pub metadata: Value,
}

/// Returns the most recent sentiment snapshot per brand.
///
/// Results are ordered by brand name ascending.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_sentiment_summary(pool: &PgPool) -> Result<Vec<SentimentSummaryRow>, DbError> {
    let rows = sqlx::query_as::<_, SentimentSummaryRow>(
        "SELECT \
             b.name AS brand_name, \
             b.slug AS brand_slug, \
             ss.score, \
             ss.signal_count, \
             ss.captured_at, \
             ss.metadata \
         FROM ( \
             SELECT DISTINCT ON (brand_id) \
                 brand_id, score, signal_count, captured_at, metadata, id \
             FROM sentiment_snapshots \
             ORDER BY brand_id, captured_at DESC, id DESC \
         ) ss \
         JOIN brands b ON b.id = ss.brand_id \
         WHERE b.deleted_at IS NULL \
           AND b.is_active = TRUE \
         ORDER BY b.name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Returns recent sentiment snapshots with brand context.
///
/// Results are ordered by `captured_at DESC`.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn list_sentiment_snapshots_dashboard(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<SentimentSnapshotDashboardRow>, DbError> {
    let rows = sqlx::query_as::<_, SentimentSnapshotDashboardRow>(
        "SELECT \
             b.name  AS brand_name, \
             b.slug  AS brand_slug, \
             ss.score, \
             ss.signal_count, \
             ss.captured_at, \
             ss.metadata \
         FROM sentiment_snapshots ss \
         JOIN brands b ON b.id = ss.brand_id \
         WHERE b.deleted_at IS NULL \
           AND b.is_active = TRUE \
         ORDER BY ss.captured_at DESC, ss.id DESC \
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
