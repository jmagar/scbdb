//! Write operations for the `store_locations` table.

use sqlx::PgPool;

use super::types::NewStoreLocation;

const UPSERT_STORE_LOCATIONS_SQL: &str = "INSERT INTO store_locations \
     (brand_id, location_key, name, address_line1, city, state, zip, country, \
      latitude, longitude, phone, external_id, locator_source, raw_data) \
 SELECT \
     $1, \
     u.location_key, \
     u.name, \
     u.address_line1, \
     u.city, \
     u.state, \
     u.zip, \
     u.country, \
     u.latitude::NUMERIC(9,6), \
     u.longitude::NUMERIC(9,6), \
     u.phone, \
     u.external_id, \
     u.locator_source, \
     u.raw_data \
 FROM UNNEST(\
      $2::text[], \
      $3::text[], \
      $4::text[], \
      $5::text[], \
      $6::text[], \
      $7::text[], \
      $8::text[], \
      $9::float8[], \
      $10::float8[], \
      $11::text[], \
      $12::text[], \
      $13::text[], \
      $14::jsonb[]) \
 AS u(\
      location_key, \
      name, \
      address_line1, \
      city, \
      state, \
      zip, \
      country, \
      latitude, \
      longitude, \
      phone, \
      external_id, \
      locator_source, \
      raw_data) \
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
 RETURNING (xmax = 0) AS is_new";

struct StoreLocationBatch {
    location_keys: Vec<String>,
    names: Vec<String>,
    address_line1s: Vec<Option<String>>,
    cities: Vec<Option<String>>,
    states: Vec<Option<String>>,
    zips: Vec<Option<String>>,
    countries: Vec<String>,
    latitudes: Vec<Option<f64>>,
    longitudes: Vec<Option<f64>>,
    phones: Vec<Option<String>>,
    external_ids: Vec<Option<String>>,
    locator_sources: Vec<Option<String>>,
    raw_datas: Vec<serde_json::Value>,
}

impl StoreLocationBatch {
    fn from_locations(locations: &[NewStoreLocation]) -> Self {
        let mut batch = Self {
            location_keys: Vec::with_capacity(locations.len()),
            names: Vec::with_capacity(locations.len()),
            address_line1s: Vec::with_capacity(locations.len()),
            cities: Vec::with_capacity(locations.len()),
            states: Vec::with_capacity(locations.len()),
            zips: Vec::with_capacity(locations.len()),
            countries: Vec::with_capacity(locations.len()),
            latitudes: Vec::with_capacity(locations.len()),
            longitudes: Vec::with_capacity(locations.len()),
            phones: Vec::with_capacity(locations.len()),
            external_ids: Vec::with_capacity(locations.len()),
            locator_sources: Vec::with_capacity(locations.len()),
            raw_datas: Vec::with_capacity(locations.len()),
        };

        for loc in locations {
            batch.location_keys.push(loc.location_key.clone());
            batch.names.push(loc.name.clone());
            batch.address_line1s.push(loc.address_line1.clone());
            batch.cities.push(loc.city.clone());
            batch.states.push(loc.state.clone());
            batch.zips.push(loc.zip.clone());
            batch
                .countries
                .push(loc.country.as_deref().unwrap_or("US").to_string());
            batch.latitudes.push(loc.latitude);
            batch.longitudes.push(loc.longitude);
            batch.phones.push(loc.phone.clone());
            batch.external_ids.push(loc.external_id.clone());
            batch.locator_sources.push(loc.locator_source.clone());
            batch.raw_datas.push(loc.raw_data.clone());
        }

        batch
    }
}

async fn run_store_location_upsert(
    pool: &PgPool,
    brand_id: i64,
    batch: &StoreLocationBatch,
) -> Result<Vec<bool>, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(UPSERT_STORE_LOCATIONS_SQL)
        .bind(brand_id)
        .bind(&batch.location_keys)
        .bind(&batch.names)
        .bind(&batch.address_line1s)
        .bind(&batch.cities)
        .bind(&batch.states)
        .bind(&batch.zips)
        .bind(&batch.countries)
        .bind(&batch.latitudes)
        .bind(&batch.longitudes)
        .bind(&batch.phones)
        .bind(&batch.external_ids)
        .bind(&batch.locator_sources)
        .bind(&batch.raw_datas)
        .fetch_all(pool)
        .await
}

/// Insert new locations and update `last_seen_at` for existing ones.
///
/// Returns `(new_count, updated_count)` where:
/// - `new_count`: rows that did not exist before (were inserted)
/// - `updated_count`: rows that already existed (were updated)
///
/// Uses a single `INSERT … SELECT FROM UNNEST(…) ON CONFLICT` so that
/// the entire batch is upserted in one round-trip regardless of batch size.
///
/// Latitude and longitude are bound as `Option<f64>` slices and cast to
/// `NUMERIC(9,6)` inside the SQL statement so that the database engine
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

    let batch = StoreLocationBatch::from_locations(locations);
    let rows = run_store_location_upsert(pool, brand_id, &batch).await?;

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
