//! Write operations for the `store_locations` table.

use sqlx::PgPool;

use super::types::NewStoreLocation;

/// Insert new locations and update `last_seen_at` for existing ones.
///
/// Returns `(new_count, updated_count)` where:
/// - `new_count`: rows that did not exist before (were inserted)
/// - `updated_count`: rows that already existed (were updated)
///
/// Uses a single `INSERT … SELECT * FROM UNNEST(…) ON CONFLICT` so that
/// the entire batch is upserted in one round-trip regardless of batch size.
///
/// Latitude and longitude are bound as `Option<f64>` slices and cast to
/// `NUMERIC(9,6)[]` inside the SQL statement so that the database engine
/// performs the type coercion consistently (matching the pattern used in
/// `upsert_variant` for dosage/size columns).
///
/// # Errors
///
/// Returns [`sqlx::Error`] if the query fails.
pub async fn upsert_store_locations(
    pool: &PgPool,
    brand_id: i64,
    locations: &[NewStoreLocation],
) -> Result<(u64, u64), sqlx::Error> {
    if locations.is_empty() {
        return Ok((0, 0));
    }

    // Collect each column into a parallel Vec for UNNEST binding.
    let mut location_keys: Vec<String> = Vec::with_capacity(locations.len());
    let mut names: Vec<String> = Vec::with_capacity(locations.len());
    let mut address_line1s: Vec<Option<String>> = Vec::with_capacity(locations.len());
    let mut cities: Vec<Option<String>> = Vec::with_capacity(locations.len());
    let mut states: Vec<Option<String>> = Vec::with_capacity(locations.len());
    let mut zips: Vec<Option<String>> = Vec::with_capacity(locations.len());
    let mut countries: Vec<String> = Vec::with_capacity(locations.len());
    let mut latitudes: Vec<Option<f64>> = Vec::with_capacity(locations.len());
    let mut longitudes: Vec<Option<f64>> = Vec::with_capacity(locations.len());
    let mut phones: Vec<Option<String>> = Vec::with_capacity(locations.len());
    let mut external_ids: Vec<Option<String>> = Vec::with_capacity(locations.len());
    let mut locator_sources: Vec<Option<String>> = Vec::with_capacity(locations.len());
    let mut raw_datas: Vec<serde_json::Value> = Vec::with_capacity(locations.len());

    for loc in locations {
        location_keys.push(loc.location_key.clone());
        names.push(loc.name.clone());
        address_line1s.push(loc.address_line1.clone());
        cities.push(loc.city.clone());
        states.push(loc.state.clone());
        zips.push(loc.zip.clone());
        countries.push(loc.country.as_deref().unwrap_or("US").to_string());
        latitudes.push(loc.latitude);
        longitudes.push(loc.longitude);
        phones.push(loc.phone.clone());
        external_ids.push(loc.external_id.clone());
        locator_sources.push(loc.locator_source.clone());
        raw_datas.push(loc.raw_data.clone());
    }

    let rows: Vec<bool> = sqlx::query_scalar::<_, bool>(
        "INSERT INTO store_locations \
             (brand_id, location_key, name, address_line1, city, state, zip, country, \
              latitude, longitude, phone, external_id, locator_source, raw_data) \
         SELECT $1, * FROM UNNEST(\
              $2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[], $8::text[], \
              $9::float8[], $10::float8[], $11::text[], $12::text[], $13::text[], $14::jsonb[]) \
         ON CONFLICT (brand_id, location_key) DO UPDATE SET \
             last_seen_at    = NOW(), \
             is_active       = TRUE, \
             updated_at      = NOW(), \
             name            = EXCLUDED.name, \
             address_line1   = EXCLUDED.address_line1, \
             city            = EXCLUDED.city, \
             state           = EXCLUDED.state, \
             zip             = EXCLUDED.zip, \
             country         = EXCLUDED.country, \
             latitude        = EXCLUDED.latitude, \
             longitude       = EXCLUDED.longitude, \
             phone           = EXCLUDED.phone, \
             external_id     = EXCLUDED.external_id, \
             locator_source  = EXCLUDED.locator_source, \
             raw_data        = EXCLUDED.raw_data \
         RETURNING (xmax = 0) AS is_new",
    )
    .bind(brand_id)
    .bind(&location_keys)
    .bind(&names)
    .bind(&address_line1s)
    .bind(&cities)
    .bind(&states)
    .bind(&zips)
    .bind(&countries)
    .bind(&latitudes)
    .bind(&longitudes)
    .bind(&phones)
    .bind(&external_ids)
    .bind(&locator_sources)
    .bind(&raw_datas)
    .fetch_all(pool)
    .await?;

    let new_count = rows.iter().filter(|&&is_new| is_new).count() as u64;
    let updated_count = rows.len() as u64 - new_count;

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
