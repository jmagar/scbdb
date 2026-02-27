//! Brand entity write handlers: create, update, deactivate.
//! Enrichment handlers (profile, social, domains) live in `write_enrichment`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;

use crate::middleware::RequestId;

use super::super::{map_db_error, ApiError, ApiResponse, AppState, ResponseMeta};
use super::{parse_url_or_validation_error, resolve_brand};

// ---------------------------------------------------------------------------
// Request bodies
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(in crate::api) struct CreateBrandRequest {
    pub name: String,
    pub relationship: String,
    pub tier: i16,
    pub domain: Option<String>,
    pub shop_url: Option<String>,
    pub store_locator_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub notes: Option<String>,
}

// Option<Option<T>> is intentional: outer None = "not in request" (keep current),
// Some(None) = "explicitly cleared", Some(Some(v)) = "set to value" (PATCH semantics).
#[allow(clippy::option_option)]
#[derive(Debug, Deserialize)]
pub(in crate::api) struct UpdateBrandRequest {
    pub name: Option<String>,
    pub relationship: Option<String>,
    pub tier: Option<i16>,
    pub domain: Option<Option<String>>,
    pub shop_url: Option<Option<String>>,
    pub store_locator_url: Option<Option<String>>,
    pub twitter_handle: Option<Option<String>>,
    pub notes: Option<Option<String>>,
}

// ---------------------------------------------------------------------------
// Response bodies
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub(in crate::api) struct CreateBrandResponse {
    pub id: i64,
    pub slug: String,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_relationship(req_id: &str, value: &str) -> Result<(), ApiError> {
    match value {
        "portfolio" | "competitor" => Ok(()),
        _ => Err(ApiError::new(
            req_id,
            "validation_error",
            format!("relationship must be 'portfolio' or 'competitor', got '{value}'"),
        )),
    }
}

fn validate_tier(req_id: &str, value: i16) -> Result<(), ApiError> {
    if matches!(value, 1..=3) {
        Ok(())
    } else {
        Err(ApiError::new(
            req_id,
            "validation_error",
            format!("tier must be 1, 2, or 3, got {value}"),
        ))
    }
}

fn validate_url_if_present(req_id: &str, field: &str, value: &str) -> Result<(), ApiError> {
    parse_url_or_validation_error(req_id, value, |v| {
        format!("'{field}' must be a valid URL, got '{v}'")
    })
    .map(|_| ())
}

fn map_unique_violation(req_id: &str, e: &scbdb_db::DbError) -> ApiError {
    if let scbdb_db::DbError::Sqlx(sqlx::Error::Database(db_err)) = e {
        if db_err.code().as_deref() == Some("23505") {
            return ApiError::new(req_id, "conflict", "a brand with that slug already exists");
        }
    }
    map_db_error(req_id.to_owned(), e)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/brands — create a new brand.
pub(in crate::api) async fn create_brand(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Json(body): Json<CreateBrandRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CreateBrandResponse>>), ApiError> {
    let rid = &req_id.0;

    let name = body.name.trim().to_owned();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::new(
            rid,
            "validation_error",
            "name must be 1–200 characters",
        ));
    }
    validate_relationship(rid, &body.relationship)?;
    validate_tier(rid, body.tier)?;
    if let Some(ref u) = body.shop_url {
        validate_url_if_present(rid, "shop_url", u)?;
    }
    if let Some(ref u) = body.store_locator_url {
        validate_url_if_present(rid, "store_locator_url", u)?;
    }

    let slug = scbdb_core::brands::slug_from_name(&name);

    let row = scbdb_db::create_brand(
        &state.pool,
        &name,
        &slug,
        &body.relationship,
        body.tier,
        body.domain.as_deref(),
        body.shop_url.as_deref(),
        body.store_locator_url.as_deref(),
        body.twitter_handle.as_deref(),
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| map_unique_violation(rid, &e))?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse {
            data: CreateBrandResponse {
                id: row.id,
                slug: row.slug,
            },
            meta: ResponseMeta::new(req_id.0),
        }),
    ))
}

/// PATCH /api/v1/brands/:slug — update core brand metadata (sparse).
pub(in crate::api) async fn update_brand(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
    Json(body): Json<UpdateBrandRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let rid = &req_id.0;
    let brand = resolve_brand(&state.pool, &slug, rid).await?;

    let trimmed_name = body.name.as_ref().map(|n| n.trim().to_owned());
    if let Some(ref name) = trimmed_name {
        if name.is_empty() || name.len() > 200 {
            return Err(ApiError::new(
                rid,
                "validation_error",
                "name must be 1–200 characters",
            ));
        }
    }
    if let Some(ref rel) = body.relationship {
        validate_relationship(rid, rel)?;
    }
    if let Some(tier) = body.tier {
        validate_tier(rid, tier)?;
    }
    if let Some(Some(ref u)) = body.shop_url {
        validate_url_if_present(rid, "shop_url", u)?;
    }
    if let Some(Some(ref u)) = body.store_locator_url {
        validate_url_if_present(rid, "store_locator_url", u)?;
    }

    scbdb_db::update_brand(
        &state.pool,
        brand.id,
        trimmed_name.as_deref(),
        body.relationship.as_deref(),
        body.tier,
        body.domain.as_ref().map(|opt| opt.as_deref()),
        body.shop_url.as_ref().map(|opt| opt.as_deref()),
        body.store_locator_url.as_ref().map(|opt| opt.as_deref()),
        body.twitter_handle.as_ref().map(|opt| opt.as_deref()),
        body.notes.as_ref().map(|opt| opt.as_deref()),
    )
    .await
    .map_err(|e| map_db_error(rid.clone(), &e))?;

    Ok(Json(ApiResponse {
        data: serde_json::json!({ "updated": true }),
        meta: ResponseMeta::new(req_id.0),
    }))
}

/// DELETE /api/v1/brands/:slug — soft-delete a brand.
pub(in crate::api) async fn deactivate_brand(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let rid = &req_id.0;
    let brand = resolve_brand(&state.pool, &slug, rid).await?;

    scbdb_db::deactivate_brand(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(rid.clone(), &e))?;

    Ok(Json(ApiResponse {
        data: serde_json::json!({ "deactivated": true }),
        meta: ResponseMeta::new(req_id.0),
    }))
}
