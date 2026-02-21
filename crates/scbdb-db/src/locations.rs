//! Database operations for the `store_locations` table.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

/// Input record for inserting/upserting a store location.
#[derive(Debug, Clone)]
pub struct NewStoreLocation {
    pub location_key: String,
    pub name: String,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub phone: Option<String>,
    pub external_id: Option<String>,
    pub locator_source: Option<String>,
    pub raw_data: serde_json::Value,
}

/// A row from the `store_locations` table.
///
/// `raw_data` is omitted — it is write-only operational storage and not
/// needed in read-back queries.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StoreLocationRow {
    pub id: i64,
    pub public_id: Uuid,
    pub brand_id: i64,
    pub location_key: String,
    pub name: String,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: String,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub phone: Option<String>,
    pub external_id: Option<String>,
    pub locator_source: Option<String>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

/// Insert new locations and update `last_seen_at` for existing ones.
///
/// Returns `(new_count, updated_count)` where:
/// - `new_count`: rows that did not exist before (were inserted)
/// - `updated_count`: rows that already existed (were updated)
///
/// Uses `INSERT … ON CONFLICT (brand_id, location_key) DO UPDATE` so that
/// existing rows have their `last_seen_at`, `is_active`, and mutable address
/// fields refreshed in place.
///
/// Latitude and longitude are bound as `Option<f64>` and cast to
/// `NUMERIC(9,6)` inside the SQL statement so that the database engine
/// performs the type coercion consistently (matching the pattern used in
/// `upsert_variant` for dosage/size columns).
///
/// # Errors
///
/// Returns [`sqlx::Error`] if any query fails.
pub async fn upsert_store_locations(
    pool: &PgPool,
    brand_id: i64,
    locations: &[NewStoreLocation],
) -> Result<(u64, u64), sqlx::Error> {
    let mut new_count: u64 = 0;
    let mut updated_count: u64 = 0;

    for loc in locations {
        let country = loc.country.as_deref().unwrap_or("US");

        let is_new: bool = sqlx::query_scalar::<_, bool>(
            "INSERT INTO store_locations \
                 (brand_id, location_key, name, address_line1, city, state, zip, country, \
                  latitude, longitude, phone, external_id, locator_source, raw_data) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, \
                     $9::NUMERIC(9,6), $10::NUMERIC(9,6), $11, $12, $13, $14::JSONB) \
             ON CONFLICT (brand_id, location_key) DO UPDATE SET \
                 last_seen_at  = NOW(), \
                 is_active     = TRUE, \
                 updated_at    = NOW(), \
                 name          = EXCLUDED.name, \
                 address_line1 = EXCLUDED.address_line1, \
                 city          = EXCLUDED.city, \
                 state         = EXCLUDED.state, \
                 zip           = EXCLUDED.zip, \
                 phone         = EXCLUDED.phone, \
                 external_id   = EXCLUDED.external_id \
             RETURNING (xmax = 0) AS is_new",
        )
        .bind(brand_id)
        .bind(&loc.location_key)
        .bind(&loc.name)
        .bind(&loc.address_line1)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.zip)
        .bind(country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.phone)
        .bind(&loc.external_id)
        .bind(&loc.locator_source)
        .bind(&loc.raw_data)
        .fetch_one(pool)
        .await?;

        if is_new {
            new_count += 1;
        } else {
            updated_count += 1;
        }
    }

    Ok((new_count, updated_count))
}

/// Mark locations for `brand_id` whose `location_key` is NOT in `active_keys`
/// as inactive.
///
/// Called after upsert to handle locations that disappeared from the locator.
/// Returns the number of rows deactivated.
///
/// When `active_keys` is empty, ALL active locations for the brand are
/// deactivated — this is intentional: an empty scrape result means the
/// locator returned nothing, so all previously-known locations are gone.
/// `PostgreSQL` evaluates `location_key != ALL('{}')` as `TRUE` for every row.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn deactivate_missing_locations(
    pool: &PgPool,
    brand_id: i64,
    active_keys: &[String],
) -> Result<u64, sqlx::Error> {
    let rows_affected = sqlx::query(
        "UPDATE store_locations \
         SET is_active = FALSE, updated_at = NOW() \
         WHERE brand_id = $1 \
           AND is_active = TRUE \
           AND location_key != ALL($2::text[])",
    )
    .bind(brand_id)
    .bind(active_keys)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected)
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

/// Query locations first seen since the given timestamp.
///
/// If `brand_slug` is provided, filters to that brand only; otherwise returns
/// all active locations across all brands first seen after `since`.
///
/// Results are ordered by `first_seen_at DESC`.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn list_new_locations_since(
    pool: &PgPool,
    since: DateTime<Utc>,
    brand_slug: Option<&str>,
) -> Result<Vec<StoreLocationRow>, sqlx::Error> {
    if let Some(slug) = brand_slug {
        sqlx::query_as::<_, StoreLocationRow>(
            "SELECT sl.id, sl.public_id, sl.brand_id, sl.location_key, \
                    sl.name, sl.address_line1, sl.city, sl.state, sl.zip, \
                    sl.country, sl.latitude, sl.longitude, sl.phone, \
                    sl.external_id, sl.locator_source, \
                    sl.first_seen_at, sl.last_seen_at, sl.is_active, \
                    sl.created_at, sl.updated_at \
             FROM store_locations sl \
             JOIN brands b ON b.id = sl.brand_id \
             WHERE sl.first_seen_at > $1 \
               AND sl.is_active = TRUE \
               AND b.slug = $2 \
             ORDER BY sl.first_seen_at DESC",
        )
        .bind(since)
        .bind(slug)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, StoreLocationRow>(
            "SELECT sl.id, sl.public_id, sl.brand_id, sl.location_key, \
                    sl.name, sl.address_line1, sl.city, sl.state, sl.zip, \
                    sl.country, sl.latitude, sl.longitude, sl.phone, \
                    sl.external_id, sl.locator_source, \
                    sl.first_seen_at, sl.last_seen_at, sl.is_active, \
                    sl.created_at, sl.updated_at \
             FROM store_locations sl \
             JOIN brands b ON b.id = sl.brand_id \
             WHERE sl.first_seen_at > $1 \
               AND sl.is_active = TRUE \
             ORDER BY sl.first_seen_at DESC",
        )
        .bind(since)
        .fetch_all(pool)
        .await
    }
}

/// List all active locations for a brand.
///
/// Results are ordered by `name ASC`.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn list_active_locations_by_brand(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<StoreLocationRow>, sqlx::Error> {
    sqlx::query_as::<_, StoreLocationRow>(
        "SELECT id, public_id, brand_id, location_key, \
                name, address_line1, city, state, zip, \
                country, latitude, longitude, phone, \
                external_id, locator_source, \
                first_seen_at, last_seen_at, is_active, \
                created_at, updated_at \
         FROM store_locations \
         WHERE brand_id = $1 AND is_active = TRUE \
         ORDER BY name ASC",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await
}
