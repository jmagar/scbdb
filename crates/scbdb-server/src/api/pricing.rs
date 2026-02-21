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
pub(super) struct PriceSnapshotItem {
    captured_at: DateTime<Utc>,
    currency_code: String,
    price: Decimal,
    compare_at_price: Option<Decimal>,
    variant_title: Option<String>,
    source_variant_id: String,
    product_name: String,
    brand_name: String,
    brand_slug: String,
    brand_logo_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PriceSnapshotQuery {
    pub brand_slug: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(super) struct PricingSummaryItem {
    brand_name: String,
    brand_slug: String,
    brand_logo_url: Option<String>,
    variant_count: i64,
    avg_price: Decimal,
    min_price: Decimal,
    max_price: Decimal,
    latest_capture_at: DateTime<Utc>,
}

pub(super) async fn list_pricing_snapshots(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Query(query): Query<PriceSnapshotQuery>,
) -> Result<Json<ApiResponse<Vec<PriceSnapshotItem>>>, ApiError> {
    let rows = scbdb_db::list_price_snapshots_dashboard(
        &state.pool,
        scbdb_db::PriceSnapshotFilters {
            brand_slug: query.brand_slug.as_deref(),
            from: query.from,
            to: query.to,
            limit: normalize_limit(query.limit),
        },
    )
    .await
    .map_err(|e| map_db_error(req_id.0.clone(), e))?;

    let data = rows
        .into_iter()
        .map(|row| PriceSnapshotItem {
            captured_at: row.captured_at,
            currency_code: row.currency_code,
            price: row.price,
            compare_at_price: row.compare_at_price,
            variant_title: row.variant_title,
            source_variant_id: row.source_variant_id,
            product_name: row.product_name,
            brand_name: row.brand_name,
            brand_slug: row.brand_slug,
            brand_logo_url: row.brand_logo_url,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(super) async fn list_pricing_summary(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
) -> Result<Json<ApiResponse<Vec<PricingSummaryItem>>>, ApiError> {
    let rows = scbdb_db::list_pricing_summary(&state.pool)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), e))?;

    let data = rows
        .into_iter()
        .map(|row| PricingSummaryItem {
            brand_name: row.brand_name,
            brand_slug: row.brand_slug,
            brand_logo_url: row.brand_logo_url,
            variant_count: row.variant_count,
            avg_price: row.avg_price,
            min_price: row.min_price,
            max_price: row.max_price,
            latest_capture_at: row.latest_capture_at,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}
