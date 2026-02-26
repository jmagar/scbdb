use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::middleware::RequestId;

use super::{map_db_error, normalize_limit, ApiError, ApiResponse, AppState, ResponseMeta};

#[derive(Debug, Serialize)]
pub(super) struct ProductItem {
    product_id: i64,
    product_name: String,
    product_status: Option<String>,
    vendor: Option<String>,
    source_url: Option<String>,
    primary_image_url: Option<String>,
    brand_name: String,
    brand_slug: String,
    brand_logo_url: Option<String>,
    relationship: String,
    tier: i16,
    variant_count: i64,
    latest_price: Option<Decimal>,
    latest_price_captured_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ProductQuery {
    pub brand_slug: Option<String>,
    pub relationship: Option<String>,
    pub tier: Option<i16>,
    pub limit: Option<i64>,
}

pub(super) async fn list_products(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Query(query): Query<ProductQuery>,
) -> Result<Json<ApiResponse<Vec<ProductItem>>>, ApiError> {
    let rows = scbdb_db::list_products_dashboard(
        &state.pool,
        scbdb_db::ProductListFilters {
            brand_slug: query.brand_slug.as_deref(),
            relationship: query.relationship.as_deref(),
            tier: query.tier,
            limit: Some(normalize_limit(query.limit)),
        },
    )
    .await
    .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let data = rows
        .into_iter()
        .map(|row| ProductItem {
            product_id: row.product_id,
            product_name: row.product_name,
            product_status: row.product_status,
            vendor: row.vendor,
            source_url: row.source_url,
            primary_image_url: row.primary_image_url,
            brand_name: row.brand_name,
            brand_slug: row.brand_slug,
            brand_logo_url: row.brand_logo_url,
            relationship: row.relationship,
            tier: row.tier,
            variant_count: row.variant_count,
            latest_price: row.latest_price,
            latest_price_captured_at: row.latest_price_captured_at,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

#[derive(Debug, Serialize)]
pub(super) struct ProductVariantItem {
    variant_id: i64,
    source_variant_id: String,
    sku: Option<String>,
    title: Option<String>,
    is_default: bool,
    is_available: bool,
    dosage_mg: Option<Decimal>,
    cbd_mg: Option<Decimal>,
    size_value: Option<Decimal>,
    size_unit: Option<String>,
    latest_price: Option<Decimal>,
    latest_compare_at_price: Option<Decimal>,
    latest_currency_code: Option<String>,
    latest_price_source_url: Option<String>,
    latest_price_captured_at: Option<DateTime<Utc>>,
}

pub(super) async fn get_product(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(product_id): Path<i64>,
) -> Result<Json<ApiResponse<ProductItem>>, ApiError> {
    let row = scbdb_db::get_product_dashboard(&state.pool, product_id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let Some(row) = row else {
        return Err(ApiError::new(req_id.0, "not_found", "product not found"));
    };

    let data = ProductItem {
        product_id: row.product_id,
        product_name: row.product_name,
        product_status: row.product_status,
        vendor: row.vendor,
        source_url: row.source_url,
        primary_image_url: row.primary_image_url,
        brand_name: row.brand_name,
        brand_slug: row.brand_slug,
        brand_logo_url: row.brand_logo_url,
        relationship: row.relationship,
        tier: row.tier,
        variant_count: row.variant_count,
        latest_price: row.latest_price,
        latest_price_captured_at: row.latest_price_captured_at,
    };

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(super) async fn list_product_variants(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(product_id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<ProductVariantItem>>>, ApiError> {
    let product = scbdb_db::get_product_dashboard(&state.pool, product_id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    if product.is_none() {
        return Err(ApiError::new(req_id.0, "not_found", "product not found"));
    }

    let rows = scbdb_db::list_product_variants_dashboard(&state.pool, product_id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let data = rows
        .into_iter()
        .map(|row| ProductVariantItem {
            variant_id: row.variant_id,
            source_variant_id: row.source_variant_id,
            sku: row.sku,
            title: row.title,
            is_default: row.is_default,
            is_available: row.is_available,
            dosage_mg: row.dosage_mg,
            cbd_mg: row.cbd_mg,
            size_value: row.size_value,
            size_unit: row.size_unit,
            latest_price: row.latest_price,
            latest_compare_at_price: row.latest_compare_at_price,
            latest_currency_code: row.latest_currency_code,
            latest_price_source_url: row.latest_price_source_url,
            latest_price_captured_at: row.latest_price_captured_at,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

#[cfg(test)]
mod tests {
    use super::ProductVariantItem;

    #[test]
    fn product_variant_item_is_serializable() {
        let item = ProductVariantItem {
            variant_id: 42,
            source_variant_id: "variant-42".to_string(),
            sku: Some("sku-42".to_string()),
            title: Some("Default".to_string()),
            is_default: true,
            is_available: true,
            dosage_mg: None,
            cbd_mg: None,
            size_value: None,
            size_unit: None,
            latest_price: None,
            latest_compare_at_price: None,
            latest_currency_code: Some("USD".to_string()),
            latest_price_source_url: Some("https://example.com".to_string()),
            latest_price_captured_at: None,
        };

        let json = serde_json::to_string(&item).expect("serialize product variant");
        assert!(json.contains("\"source_variant_id\":\"variant-42\""));
        assert!(json.contains("\"latest_currency_code\":\"USD\""));
    }
}
