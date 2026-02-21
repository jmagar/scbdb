//! F1: GET /api/v1/brands — brand list with completeness scores.

use axum::{extract::State, Extension, Json};
use serde::Serialize;

use crate::middleware::RequestId;

use super::super::{map_db_error, ApiResponse, AppState, ResponseMeta};

#[derive(Debug, Serialize)]
pub(in crate::api) struct BrandSummaryItem {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub relationship: String,
    pub tier: i16,
    pub logo_url: Option<String>,
    pub completeness_score: i32,
}

pub(in crate::api) async fn list_brands(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
) -> Result<Json<ApiResponse<Vec<BrandSummaryItem>>>, super::super::ApiError> {
    let (brands, completeness_map) = tokio::try_join!(
        scbdb_db::list_active_brands(&state.pool),
        scbdb_db::get_all_brands_completeness(&state.pool),
    )
    .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let data = brands
        .into_iter()
        .map(|brand| {
            let score = completeness_map.get(&brand.id).copied().unwrap_or(0);
            BrandSummaryItem {
                id: brand.id,
                slug: brand.slug,
                name: brand.name,
                relationship: brand.relationship,
                tier: brand.tier,
                logo_url: brand.logo_url,
                completeness_score: score,
            }
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}
