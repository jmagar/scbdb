//! F4: GET /api/v1/brands/:slug/funding|lab-tests|legal|sponsorships|distributors|competitors|media
//!
//! Seven brand intel endpoints. Response item types live in `types`.

mod types;

pub(in crate::api) use types::{
    CompetitorItem, DistributorItem, FundingEventItem, LabTestItem, LegalProceedingItem,
    MediaAppearanceItem, SponsorshipItem,
};

use axum::{
    extract::{Path, State},
    Extension, Json,
};

use crate::middleware::RequestId;

use super::super::{map_db_error, ApiError, ApiResponse, AppState, ResponseMeta};
use super::resolve_brand;

pub(in crate::api) async fn list_funding(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<Vec<FundingEventItem>>>, ApiError> {
    let brand = resolve_brand(&state.pool, &slug, &req_id.0).await?;
    let rows = scbdb_db::list_brand_funding_events(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;
    let data = rows
        .into_iter()
        .map(|r| FundingEventItem {
            id: r.id,
            event_type: r.event_type,
            amount_usd: r.amount_usd,
            announced_at: r.announced_at,
            investors: r.investors,
            acquirer: r.acquirer,
            source_url: r.source_url,
            notes: r.notes,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(in crate::api) async fn list_lab_tests(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<Vec<LabTestItem>>>, ApiError> {
    let brand = resolve_brand(&state.pool, &slug, &req_id.0).await?;
    let rows = scbdb_db::list_brand_lab_tests(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;
    let data = rows
        .into_iter()
        .map(|r| LabTestItem {
            id: r.id,
            product_id: r.product_id,
            variant_id: r.variant_id,
            lab_name: r.lab_name,
            test_date: r.test_date,
            report_url: r.report_url,
            thc_mg_actual: r.thc_mg_actual,
            cbd_mg_actual: r.cbd_mg_actual,
            total_cannabinoids_mg: r.total_cannabinoids_mg,
            passed: r.passed,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(in crate::api) async fn list_legal(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<Vec<LegalProceedingItem>>>, ApiError> {
    let brand = resolve_brand(&state.pool, &slug, &req_id.0).await?;
    let rows = scbdb_db::list_brand_legal_proceedings(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;
    let data = rows
        .into_iter()
        .map(|r| LegalProceedingItem {
            id: r.id,
            proceeding_type: r.proceeding_type,
            jurisdiction: r.jurisdiction,
            case_number: r.case_number,
            title: r.title,
            summary: r.summary,
            status: r.status,
            filed_at: r.filed_at,
            resolved_at: r.resolved_at,
            source_url: r.source_url,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(in crate::api) async fn list_sponsorships(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<Vec<SponsorshipItem>>>, ApiError> {
    let brand = resolve_brand(&state.pool, &slug, &req_id.0).await?;
    let rows = scbdb_db::list_brand_sponsorships(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;
    let data = rows
        .into_iter()
        .map(|r| SponsorshipItem {
            id: r.id,
            entity_name: r.entity_name,
            entity_type: r.entity_type,
            deal_type: r.deal_type,
            announced_at: r.announced_at,
            ends_at: r.ends_at,
            source_url: r.source_url,
            notes: r.notes,
            is_active: r.is_active,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(in crate::api) async fn list_distributors(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<Vec<DistributorItem>>>, ApiError> {
    let brand = resolve_brand(&state.pool, &slug, &req_id.0).await?;
    let rows = scbdb_db::list_brand_distributors(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;
    let data = rows
        .into_iter()
        .map(|r| DistributorItem {
            id: r.id,
            distributor_name: r.distributor_name,
            distributor_slug: r.distributor_slug,
            states: r.states,
            territory_type: r.territory_type,
            channel_type: r.channel_type,
            started_at: r.started_at,
            ended_at: r.ended_at,
            is_active: r.is_active,
            notes: r.notes,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(in crate::api) async fn list_competitors(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<Vec<CompetitorItem>>>, ApiError> {
    let brand = resolve_brand(&state.pool, &slug, &req_id.0).await?;
    let rows = scbdb_db::list_brand_competitor_relationships(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;
    let data = rows
        .into_iter()
        .map(|r| CompetitorItem {
            id: r.id,
            brand_id: r.brand_id,
            competitor_brand_id: r.competitor_brand_id,
            relationship_type: r.relationship_type,
            distributor_name: r.distributor_name,
            states: r.states,
            notes: r.notes,
            first_observed_at: r.first_observed_at,
            is_active: r.is_active,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(in crate::api) async fn list_media(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<Vec<MediaAppearanceItem>>>, ApiError> {
    let brand = resolve_brand(&state.pool, &slug, &req_id.0).await?;
    let rows = scbdb_db::list_brand_media_appearances(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;
    let data = rows
        .into_iter()
        .map(|r| MediaAppearanceItem {
            id: r.id,
            brand_signal_id: r.brand_signal_id,
            appearance_type: r.appearance_type,
            outlet_name: r.outlet_name,
            title: r.title,
            host_or_author: r.host_or_author,
            aired_at: r.aired_at,
            duration_seconds: r.duration_seconds,
            source_url: r.source_url,
            notes: r.notes,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}
