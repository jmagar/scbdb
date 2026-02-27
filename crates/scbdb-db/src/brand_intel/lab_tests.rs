//! Database operations for the `brand_lab_tests` table.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::DbError;

/// A row from the `brand_lab_tests` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandLabTestRow {
    pub id: i64,
    pub brand_id: i64,
    pub product_id: Option<i64>,
    pub variant_id: Option<i64>,
    pub lab_name: Option<String>,
    pub test_date: Option<NaiveDate>,
    pub report_url: Option<String>,
    pub thc_mg_actual: Option<Decimal>,
    pub cbd_mg_actual: Option<Decimal>,
    pub total_cannabinoids_mg: Option<Decimal>,
    pub passed: Option<bool>,
    pub raw_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields required to insert a new `brand_lab_tests` row.
#[derive(Debug)]
pub struct NewBrandLabTest<'a> {
    pub brand_id: i64,
    pub product_id: Option<i64>,
    pub variant_id: Option<i64>,
    pub lab_name: Option<&'a str>,
    pub test_date: Option<NaiveDate>,
    pub report_url: Option<&'a str>,
    pub thc_mg_actual: Option<Decimal>,
    pub cbd_mg_actual: Option<Decimal>,
    pub total_cannabinoids_mg: Option<Decimal>,
    pub passed: Option<bool>,
    pub raw_data: Option<&'a serde_json::Value>,
}

/// List all lab tests for a brand, most recent first.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn list_brand_lab_tests(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Vec<BrandLabTestRow>, DbError> {
    Ok(sqlx::query_as::<_, BrandLabTestRow>(
        "SELECT id, brand_id, product_id, variant_id, lab_name, test_date, \
                report_url, thc_mg_actual, cbd_mg_actual, total_cannabinoids_mg, \
                passed, raw_data, created_at, updated_at \
         FROM brand_lab_tests \
         WHERE brand_id = $1 \
         ORDER BY test_date DESC NULLS LAST, id DESC",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Insert a lab test. Returns the generated row `id`.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn insert_brand_lab_test(
    pool: &PgPool,
    test: &NewBrandLabTest<'_>,
) -> Result<i64, DbError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO brand_lab_tests \
           (brand_id, product_id, variant_id, lab_name, test_date, report_url, \
            thc_mg_actual, cbd_mg_actual, total_cannabinoids_mg, passed, raw_data) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         RETURNING id",
    )
    .bind(test.brand_id)
    .bind(test.product_id)
    .bind(test.variant_id)
    .bind(test.lab_name)
    .bind(test.test_date)
    .bind(test.report_url)
    .bind(test.thc_mg_actual)
    .bind(test.cbd_mg_actual)
    .bind(test.total_cannabinoids_mg)
    .bind(test.passed)
    .bind(test.raw_data)
    .fetch_one(pool)
    .await?;
    Ok(id)
}
