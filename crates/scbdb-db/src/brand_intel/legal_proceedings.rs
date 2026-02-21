//! Database operations for the `brand_legal_proceedings` table.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::DbError;

/// A row from the `brand_legal_proceedings` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandLegalProceedingRow {
    pub id: i64,
    pub brand_id: i64,
    pub proceeding_type: String,
    pub jurisdiction: Option<String>,
    pub case_number: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub status: String,
    pub filed_at: Option<NaiveDate>,
    pub resolved_at: Option<NaiveDate>,
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields required to insert a new `brand_legal_proceedings` row.
#[derive(Debug)]
pub struct NewBrandLegalProceeding<'a> {
    pub brand_id: i64,
    pub proceeding_type: &'a str,
    pub jurisdiction: Option<&'a str>,
    pub case_number: Option<&'a str>,
    pub title: &'a str,
    pub summary: Option<&'a str>,
    pub status: &'a str,
    pub filed_at: Option<NaiveDate>,
    pub resolved_at: Option<NaiveDate>,
    pub source_url: Option<&'a str>,
}

/// List all legal proceedings for a brand, most recently filed first.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brand_legal_proceedings(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<BrandLegalProceedingRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandLegalProceedingRow>(
        "SELECT id, brand_id, proceeding_type, jurisdiction, case_number, \
                title, summary, status, filed_at, resolved_at, source_url, \
                created_at, updated_at \
         FROM brand_legal_proceedings \
         WHERE brand_id = $1 \
         ORDER BY filed_at DESC NULLS LAST, id DESC",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Insert a legal proceeding. Returns the generated row `id`.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn insert_brand_legal_proceeding(
    pool: &PgPool,
    proceeding: &NewBrandLegalProceeding<'_>,
) -> Result<i64, DbError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO brand_legal_proceedings \
           (brand_id, proceeding_type, jurisdiction, case_number, title, \
            summary, status, filed_at, resolved_at, source_url) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING id",
    )
    .bind(proceeding.brand_id)
    .bind(proceeding.proceeding_type)
    .bind(proceeding.jurisdiction)
    .bind(proceeding.case_number)
    .bind(proceeding.title)
    .bind(proceeding.summary)
    .bind(proceeding.status)
    .bind(proceeding.filed_at)
    .bind(proceeding.resolved_at)
    .bind(proceeding.source_url)
    .fetch_one(pool)
    .await?;
    Ok(id)
}
