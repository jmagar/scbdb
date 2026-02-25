use axum::{
    extract::{Query, State},
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
