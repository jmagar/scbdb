//! Database operations for `products`, `product_variants`, and `price_snapshots`.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;

use crate::DbError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

/// A row from the `products` table.
///
/// `handle` is not present in the initial schema migration; it is added in a
/// subsequent Phase 2 migration. Until that migration runs, querying the
/// column will fail. The field is included here for forward-compatibility
/// but must not appear in SELECT lists against the initial schema.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProductRow {
    pub id: i64,
    pub brand_id: i64,
    pub source_platform: String,
    pub source_product_id: String,
    pub name: String,
    /// `NULL` in the initial schema; typically `"active"` or `"draft"`.
    pub status: Option<String>,
    /// Added in Phase 2 migration; excluded from queries against initial schema.
    pub handle: Option<String>,
    /// Added in Phase 2 migration (20260219000300); excluded from queries
    /// against earlier schema versions.
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A row from the `product_variants` table.
///
/// `is_available` and `title` are nullable in the initial schema.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VariantRow {
    pub id: i64,
    pub product_id: i64,
    pub source_variant_id: String,
    pub sku: Option<String>,
    /// Nullable in schema (`TEXT` with no `NOT NULL`).
    pub title: Option<String>,
    pub is_default: bool,
    /// Nullable in schema (`BOOLEAN` with no `NOT NULL`).
    pub is_available: Option<bool>,
    pub dosage_mg: Option<Decimal>,
    pub cbd_mg: Option<Decimal>,
    pub size_value: Option<Decimal>,
    pub size_unit: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A row from the `price_snapshots` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PriceSnapshotRow {
    pub id: i64,
    pub variant_id: i64,
    /// Nullable foreign key to `collection_runs`.
    pub collection_run_id: Option<i64>,
    pub captured_at: DateTime<Utc>,
    pub currency_code: String,
    pub price: Decimal,
    pub compare_at_price: Option<Decimal>,
    pub source_url: Option<String>,
}

// ---------------------------------------------------------------------------
// products operations
// ---------------------------------------------------------------------------

/// Upserts a product row.
///
/// Conflicts on `(brand_id, source_platform, source_product_id)` update
/// `name`, `description`, `status`, `product_type`, `tags`, `handle`,
/// `metadata`, and `updated_at` in place.
///
/// Returns the internal `id` of the upserted row.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the upsert fails.
pub async fn upsert_product(
    pool: &PgPool,
    brand_id: i64,
    product: &scbdb_core::NormalizedProduct,
) -> Result<i64, DbError> {
    let metadata = json!({
        "source_url": product.source_url,
    });

    let id: i64 = sqlx::query_scalar::<_, i64>(
        "INSERT INTO products \
             (brand_id, source_platform, source_product_id, name, description, status, \
              product_type, tags, handle, source_url, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, \
                 $7, $8, $9, $10, $11::jsonb) \
         ON CONFLICT (brand_id, source_platform, source_product_id) DO UPDATE SET \
             name         = EXCLUDED.name, \
             description  = EXCLUDED.description, \
             status       = EXCLUDED.status, \
             product_type = EXCLUDED.product_type, \
             tags         = EXCLUDED.tags, \
             handle       = EXCLUDED.handle, \
             source_url   = EXCLUDED.source_url, \
             metadata     = EXCLUDED.metadata, \
             updated_at   = NOW() \
         RETURNING id",
    )
    .bind(brand_id)
    .bind(&product.source_platform)
    .bind(&product.source_product_id)
    .bind(&product.name)
    .bind(&product.description)
    .bind(&product.status)
    .bind(&product.product_type)
    .bind(&product.tags)
    .bind(&product.handle)
    .bind(&product.source_url)
    .bind(metadata)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// product_variants operations
// ---------------------------------------------------------------------------

/// Upserts a variant row.
///
/// Conflicts on `(product_id, source_variant_id)` update `sku`, `title`,
/// `is_available`, `dosage_mg`, `cbd_mg`, `size_value`, `size_unit`, and
/// `updated_at` in place.
///
/// Numeric fields (`dosage_mg`, `cbd_mg`, `size_value`) are bound as `f64`
/// and cast to fixed-scale `NUMERIC` columns (`8,2`, `8,2`, and `10,2`)
/// by the database engine. This is a documented precision boundary where
/// scrape-time floating values are rounded on persistence.
///
/// Returns the internal `id` of the upserted row.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the upsert fails.
pub async fn upsert_variant(
    pool: &PgPool,
    product_id: i64,
    variant: &scbdb_core::NormalizedVariant,
) -> Result<i64, DbError> {
    let id: i64 = sqlx::query_scalar::<_, i64>(
        "INSERT INTO product_variants \
             (product_id, source_variant_id, sku, title, is_default, is_available, \
              dosage_mg, cbd_mg, size_value, size_unit) \
         VALUES ($1, $2, $3, $4, $5, $6, \
                 $7::numeric(8,2), $8::numeric(8,2), $9::numeric(10,2), $10) \
         ON CONFLICT (product_id, source_variant_id) DO UPDATE SET \
             sku          = EXCLUDED.sku, \
             title        = EXCLUDED.title, \
             is_default   = EXCLUDED.is_default, \
             is_available = EXCLUDED.is_available, \
             dosage_mg    = EXCLUDED.dosage_mg, \
             cbd_mg       = EXCLUDED.cbd_mg, \
             size_value   = EXCLUDED.size_value, \
             size_unit    = EXCLUDED.size_unit, \
             updated_at   = NOW() \
         RETURNING id",
    )
    .bind(product_id)
    .bind(&variant.source_variant_id)
    .bind(&variant.sku)
    .bind(&variant.title)
    .bind(variant.is_default)
    .bind(variant.is_available)
    .bind(variant.dosage_mg)
    .bind(variant.cbd_mg)
    .bind(variant.size_value)
    .bind(&variant.size_unit)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// price_snapshots operations
// ---------------------------------------------------------------------------

/// Returns the most recent price snapshot for a variant, if one exists.
///
/// Ordered by `captured_at DESC` so that the first row is always the latest.
/// Used to detect whether the price has changed before inserting a new
/// snapshot.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the query fails.
pub async fn get_last_price_snapshot(
    pool: &PgPool,
    variant_id: i64,
) -> Result<Option<PriceSnapshotRow>, DbError> {
    let row = sqlx::query_as::<_, PriceSnapshotRow>(
        "SELECT id, variant_id, collection_run_id, captured_at, currency_code, \
                price, compare_at_price, source_url \
         FROM price_snapshots \
         WHERE variant_id = $1 \
         ORDER BY captured_at DESC \
         LIMIT 1",
    )
    .bind(variant_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Inserts a new price snapshot only if the price differs from the last one.
///
/// The comparison is done by parsing `price` into a [`Decimal`] and comparing
/// it to the `price` column of the most recent snapshot, plus a compare-at
/// comparison. If both values are unchanged, the function returns `Ok(false)`
/// without touching the database.
///
/// The `price` and `compare_at_price` strings are bound as `TEXT` and cast
/// to `NUMERIC(10,2)` inside the SQL statement so that the database engine
/// performs the type coercion consistently.
///
/// Returns `true` if a new snapshot was inserted, `false` if the price was
/// unchanged.
///
/// `collection_run_id` is optional to support ad-hoc/manual snapshot capture
/// outside an orchestrated collection run.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] wrapping a protocol error if `price` cannot be
/// parsed as a decimal number, or if any database operation fails.
pub async fn insert_price_snapshot_if_changed(
    pool: &PgPool,
    variant_id: i64,
    collection_run_id: Option<i64>,
    price: &str,
    compare_at_price: Option<&str>,
    currency_code: &str,
    source_url: Option<&str>,
) -> Result<bool, DbError> {
    let new_price = Decimal::from_str(price)
        .map_err(|e| sqlx::Error::Protocol(format!("invalid price string '{price}': {e}")))?;
    let new_compare_at = compare_at_price
        .map(Decimal::from_str)
        .transpose()
        .map_err(|e| {
            sqlx::Error::Protocol(format!(
                "invalid compare_at_price string '{compare_at_price:?}': {e}"
            ))
        })?;

    // Short-circuit if both price and compare-at are unchanged.
    if let Some(last) = get_last_price_snapshot(pool, variant_id).await? {
        if last.price == new_price && last.compare_at_price == new_compare_at {
            return Ok(false);
        }
    }

    sqlx::query(
        "INSERT INTO price_snapshots \
             (variant_id, collection_run_id, captured_at, currency_code, \
              price, compare_at_price, source_url) \
         VALUES ($1, $2, NOW(), $3, \
                 $4::numeric(10,2), $5::numeric(10,2), $6)",
    )
    .bind(variant_id)
    .bind(collection_run_id)
    .bind(currency_code)
    .bind(price)
    .bind(compare_at_price)
    .bind(source_url)
    .execute(pool)
    .await?;

    Ok(true)
}
