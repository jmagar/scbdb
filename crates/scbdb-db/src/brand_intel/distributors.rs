//! Database operations for the `brand_distributors` table.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::DbError;

/// A row from the `brand_distributors` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandDistributorRow {
    pub id: i64,
    pub brand_id: i64,
    pub distributor_name: String,
    pub distributor_slug: String,
    pub states: Option<Vec<String>>,
    pub territory_type: String,
    pub channel_type: String,
    pub started_at: Option<NaiveDate>,
    pub ended_at: Option<NaiveDate>,
    pub is_active: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields required to insert a new `brand_distributors` row.
#[derive(Debug)]
pub struct NewBrandDistributor<'a> {
    pub brand_id: i64,
    pub distributor_name: &'a str,
    pub distributor_slug: &'a str,
    pub states: Option<&'a [String]>,
    pub territory_type: &'a str,
    pub channel_type: &'a str,
    pub started_at: Option<NaiveDate>,
    pub ended_at: Option<NaiveDate>,
    pub is_active: bool,
    pub notes: Option<&'a str>,
}

/// List all distributors for a brand, ordered by active status then name.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brand_distributors(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<BrandDistributorRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandDistributorRow>(
        "SELECT id, brand_id, distributor_name, distributor_slug, states, \
                territory_type, channel_type, started_at, ended_at, is_active, \
                notes, created_at, updated_at \
         FROM brand_distributors \
         WHERE brand_id = $1 \
         ORDER BY is_active DESC, distributor_name ASC, id DESC",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Insert a distributor. Returns the generated row `id`.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn insert_brand_distributor(
    pool: &PgPool,
    distributor: &NewBrandDistributor<'_>,
) -> Result<i64, DbError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO brand_distributors \
           (brand_id, distributor_name, distributor_slug, states, territory_type, \
            channel_type, started_at, ended_at, is_active, notes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING id",
    )
    .bind(distributor.brand_id)
    .bind(distributor.distributor_name)
    .bind(distributor.distributor_slug)
    .bind(distributor.states)
    .bind(distributor.territory_type)
    .bind(distributor.channel_type)
    .bind(distributor.started_at)
    .bind(distributor.ended_at)
    .bind(distributor.is_active)
    .bind(distributor.notes)
    .fetch_one(pool)
    .await?;
    Ok(id)
}
