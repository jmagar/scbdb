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
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_profile(rid: &str, body: &UpsertProfileRequest) -> Result<(), ApiError> {
    if let Some(ref desc) = body.description {
        if desc.len() > 10_000 {
            return Err(ApiError::new(
                rid,
                "validation_error",
                "description must be at most 10,000 characters",
            ));
        }
    }
    if let Some(ref tagline) = body.tagline {
        if tagline.len() > 500 {
            return Err(ApiError::new(
                rid,
                "validation_error",
                "tagline must be at most 500 characters",
            ));
        }
    }
    Ok(())
}

fn validate_social_handles(rid: &str, handles: &HashMap<String, String>) -> Result<(), ApiError> {
    if handles.len() > 20 {
        return Err(ApiError::new(
            rid,
            "validation_error",
            "at most 20 social handles allowed",
        ));
    }
    for (platform, handle) in handles {
        if platform.len() > 50 {
            return Err(ApiError::new(
                rid,
                "validation_error",
                format!("platform name must be at most 50 characters, got '{platform}'"),
            ));
        }
        if handle.len() > 200 {
            return Err(ApiError::new(
                rid,
                "validation_error",
                format!("handle must be at most 200 characters for platform '{platform}'"),
            ));
        }
    }
    Ok(())
}

fn validate_domains(rid: &str, domains: &[String]) -> Result<(), ApiError> {
    if domains.len() > 50 {
        return Err(ApiError::new(
            rid,
            "validation_error",
            "at most 50 domains allowed",
        ));
    }
    for domain in domains {
        if reqwest::Url::parse(domain).is_err() {
            return Err(ApiError::new(
                rid,
                "validation_error",
                format!("invalid URL: '{domain}'"),
            ));
        }
        let scheme = reqwest::Url::parse(domain)
            .map(|u| u.scheme().to_owned())
            .unwrap_or_default();
        if scheme != "http" && scheme != "https" {
            return Err(ApiError::new(
                rid,
                "validation_error",
                format!("domain must use http or https scheme, got '{scheme}' in '{domain}'"),
            ));
        }
    }
    Ok(())
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

    validate_profile(rid, &body)?;

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

    validate_social_handles(rid, &body.handles)?;

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

    validate_domains(rid, &body.domains)?;

    scbdb_db::replace_brand_domains(&state.pool, brand.id, &body.domains)
        .await
        .map_err(|e| map_db_error(rid.clone(), &e))?;

    Ok(Json(ApiResponse {
        data: serde_json::json!({ "updated": true }),
        meta: ResponseMeta::new(req_id.0),
    }))
}
