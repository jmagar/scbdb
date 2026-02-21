//! Database operations for the `brand_competitor_relationships` table.
//!
//! The underlying table enforces `CHECK (brand_id < competitor_brand_id)` to
//! ensure a canonical ordering for each pair. The [`insert_brand_competitor_relationship`]
//! function handles this transparently: callers may pass ids in any order and
//! the function will swap them if necessary before inserting.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::DbError;

/// A row from the `brand_competitor_relationships` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandCompetitorRelationshipRow {
    pub id: i64,
    pub brand_id: i64,
    pub competitor_brand_id: i64,
    pub relationship_type: String,
    pub distributor_name: Option<String>,
    pub states: Option<Vec<String>>,
    pub notes: Option<String>,
    pub first_observed_at: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields required to insert a new `brand_competitor_relationships` row.
///
/// The `brand_id` and `competitor_brand_id` may be provided in any order;
/// the insert function will canonicalize them so that `brand_id < competitor_brand_id`.
#[derive(Debug)]
pub struct NewBrandCompetitorRelationship<'a> {
    pub brand_id: i64,
    pub competitor_brand_id: i64,
    pub relationship_type: &'a str,
    pub distributor_name: Option<&'a str>,
    pub states: Option<&'a [String]>,
    pub notes: Option<&'a str>,
    pub is_active: bool,
}

/// List all competitor relationships involving a brand.
///
/// Because the table stores relationships in canonical order
/// (`brand_id < competitor_brand_id`), the given brand could appear
/// on either side. This query returns rows where the brand is in
/// either the `brand_id` or `competitor_brand_id` column.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brand_competitor_relationships(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<BrandCompetitorRelationshipRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandCompetitorRelationshipRow>(
        "SELECT id, brand_id, competitor_brand_id, relationship_type, \
                distributor_name, states, notes, first_observed_at, is_active, \
                created_at, updated_at \
         FROM brand_competitor_relationships \
         WHERE brand_id = $1 OR competitor_brand_id = $1 \
         ORDER BY is_active DESC, first_observed_at DESC, id DESC",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Insert a competitor relationship. Returns the generated row `id`.
///
/// The `brand_id` and `competitor_brand_id` are canonicalized so that
/// the lower id is always stored in `brand_id` and the higher in
/// `competitor_brand_id`, satisfying the `CHECK (brand_id < competitor_brand_id)`
/// constraint.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn insert_brand_competitor_relationship(
    pool: &PgPool,
    rel: &NewBrandCompetitorRelationship<'_>,
) -> Result<i64, DbError> {
    // Canonicalize: ensure brand_id < competitor_brand_id to satisfy the CHECK constraint.
    let (lo, hi) = if rel.brand_id < rel.competitor_brand_id {
        (rel.brand_id, rel.competitor_brand_id)
    } else {
        (rel.competitor_brand_id, rel.brand_id)
    };

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO brand_competitor_relationships \
           (brand_id, competitor_brand_id, relationship_type, distributor_name, \
            states, notes, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING id",
    )
    .bind(lo)
    .bind(hi)
    .bind(rel.relationship_type)
    .bind(rel.distributor_name)
    .bind(rel.states)
    .bind(rel.notes)
    .bind(rel.is_active)
    .fetch_one(pool)
    .await?;
    Ok(id)
}
