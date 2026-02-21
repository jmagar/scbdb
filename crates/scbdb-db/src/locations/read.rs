//! Read operations for the `store_locations` table.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::types::{LocationPinRow, LocationsByStateRow, LocationsDashboardRow, StoreLocationRow};

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

/// Return per-brand location stats for all brands with at least one active location.
///
/// Ordered by `active_count DESC`.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn list_locations_dashboard_summary(
    pool: &PgPool,
) -> Result<Vec<LocationsDashboardRow>, sqlx::Error> {
    sqlx::query_as::<_, LocationsDashboardRow>(
        "SELECT \
            b.name  AS brand_name, \
            b.slug  AS brand_slug, \
            COUNT(*) FILTER (WHERE sl.is_active = TRUE) AS active_count, \
            COUNT(*) FILTER (\
                WHERE sl.is_active = TRUE \
                  AND sl.first_seen_at > NOW() - INTERVAL '7 days'\
            ) AS new_this_week, \
            COUNT(DISTINCT sl.state) FILTER (\
                WHERE sl.is_active = TRUE \
                  AND sl.state IS NOT NULL \
                  AND sl.state != ''\
            ) AS states_covered, \
            (\
                SELECT sl2.locator_source \
                FROM store_locations sl2 \
                WHERE sl2.brand_id = b.id \
                  AND sl2.is_active = TRUE \
                  AND sl2.locator_source IS NOT NULL \
                GROUP BY sl2.locator_source \
                ORDER BY COUNT(*) DESC \
                LIMIT 1\
            ) AS locator_source, \
            MAX(sl.last_seen_at) FILTER (WHERE sl.is_active = TRUE) AS last_seen_at \
         FROM brands b \
         JOIN store_locations sl ON sl.brand_id = b.id \
         WHERE b.is_active = TRUE AND b.deleted_at IS NULL \
         GROUP BY b.id, b.name, b.slug \
         HAVING COUNT(*) FILTER (WHERE sl.is_active = TRUE) > 0 \
         ORDER BY active_count DESC",
    )
    .fetch_all(pool)
    .await
}

/// Return state-level location counts across all active locations.
///
/// Used to color the US state coverage tile map on the dashboard.
/// Ordered by `location_count DESC`.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn list_locations_by_state(
    pool: &PgPool,
) -> Result<Vec<LocationsByStateRow>, sqlx::Error> {
    sqlx::query_as::<_, LocationsByStateRow>(
        "SELECT \
            sl.state, \
            COUNT(DISTINCT sl.brand_id) AS brand_count, \
            COUNT(*) AS location_count \
         FROM store_locations sl \
         WHERE sl.is_active = TRUE \
           AND sl.state IS NOT NULL \
           AND sl.state != '' \
         GROUP BY sl.state \
         ORDER BY location_count DESC",
    )
    .fetch_all(pool)
    .await
}

/// Return all active store locations with coordinates, joined with brand info.
///
/// Used to populate the interactive map pins. Only returns locations where
/// both `latitude` and `longitude` are non-null. Results are ordered by
/// `brand_slug ASC, name ASC`.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn list_active_location_pins(pool: &PgPool) -> Result<Vec<LocationPinRow>, sqlx::Error> {
    sqlx::query_as::<_, LocationPinRow>(
        "SELECT \
            sl.latitude::float8 AS latitude, \
            sl.longitude::float8 AS longitude, \
            sl.name AS store_name, \
            sl.address_line1, sl.city, sl.state, sl.zip, sl.locator_source, \
            b.name AS brand_name, \
            b.slug AS brand_slug, \
            b.relationship AS brand_relationship, \
            b.tier AS brand_tier \
         FROM store_locations sl \
         JOIN brands b ON b.id = sl.brand_id \
         WHERE sl.is_active = TRUE \
           AND sl.latitude IS NOT NULL \
           AND sl.longitude IS NOT NULL \
           AND b.is_active = TRUE \
           AND b.deleted_at IS NULL \
         ORDER BY b.slug ASC, sl.name ASC",
    )
    .fetch_all(pool)
    .await
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

/// Return the set of active `location_key` values for a brand.
///
/// Call this **before** [`crate::upsert_store_locations`] to enable
/// before/after diffing: subtract the new key set from the returned set to
/// find removed locations and vice-versa for additions.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn get_active_location_keys_for_brand(
    pool: &PgPool,
    brand_id: i64,
) -> Result<HashSet<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT location_key FROM store_locations WHERE brand_id = $1 AND is_active = TRUE",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(k,)| k).collect())
}
