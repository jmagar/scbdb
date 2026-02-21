//! Brand enrichment write handlers: profile, social handles, domains.

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::middleware::RequestId;

use super::super::{map_db_error, ApiError, ApiResponse, AppState, ResponseMeta};
use super::resolve_brand;

// ---------------------------------------------------------------------------
// Request bodies
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(in crate::api) struct UpsertProfileRequest {
    pub tagline: Option<String>,
    pub description: Option<String>,
    pub founded_year: Option<i16>,
    pub hq_city: Option<String>,
    pub hq_state: Option<String>,
    pub ceo_name: Option<String>,
    pub funding_stage: Option<String>,
    pub employee_count_approx: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub(in crate::api) struct UpsertSocialRequest {
    pub handles: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(in crate::api) struct UpsertDomainsRequest {
    pub domains: Vec<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// PUT /api/v1/brands/:slug/profile — overwrite brand profile fields.
pub(in crate::api) async fn upsert_brand_profile(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
    Json(body): Json<UpsertProfileRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let rid = &req_id.0;
    let brand = resolve_brand(&state.pool, &slug, rid).await?;

    scbdb_db::overwrite_brand_profile(
        &state.pool,
        brand.id,
        body.tagline.as_deref(),
        body.description.as_deref(),
        body.founded_year,
        body.hq_city.as_deref(),
        body.hq_state.as_deref(),
        body.ceo_name.as_deref(),
        body.funding_stage.as_deref(),
        body.employee_count_approx,
    )
    .await
    .map_err(|e| map_db_error(rid.clone(), &e))?;

    Ok(Json(ApiResponse {
        data: serde_json::json!({ "updated": true }),
        meta: ResponseMeta::new(req_id.0),
    }))
}

/// PUT /api/v1/brands/:slug/social — replace brand social handles.
pub(in crate::api) async fn upsert_brand_social(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
    Json(body): Json<UpsertSocialRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let rid = &req_id.0;
    let brand = resolve_brand(&state.pool, &slug, rid).await?;

    scbdb_db::replace_brand_social_handles(&state.pool, brand.id, &body.handles)
        .await
        .map_err(|e| map_db_error(rid.clone(), &e))?;

    Ok(Json(ApiResponse {
        data: serde_json::json!({ "updated": true }),
        meta: ResponseMeta::new(req_id.0),
    }))
}

/// PUT /api/v1/brands/:slug/domains — replace brand domains.
pub(in crate::api) async fn upsert_brand_domains(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
    Json(body): Json<UpsertDomainsRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let rid = &req_id.0;
    let brand = resolve_brand(&state.pool, &slug, rid).await?;

    scbdb_db::replace_brand_domains(&state.pool, brand.id, &body.domains)
        .await
        .map_err(|e| map_db_error(rid.clone(), &e))?;

    Ok(Json(ApiResponse {
        data: serde_json::json!({ "updated": true }),
        meta: ResponseMeta::new(req_id.0),
    }))
}
