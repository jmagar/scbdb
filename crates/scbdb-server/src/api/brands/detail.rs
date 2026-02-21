//! F2: GET /api/v1/brands/:slug — full brand profile.

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde::Serialize;

use crate::middleware::RequestId;

use super::super::{map_db_error, ApiError, ApiResponse, AppState, ResponseMeta};
use super::resolve_brand;

#[derive(Debug, Serialize)]
pub(in crate::api) struct BrandProfileResponse {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub relationship: String,
    pub tier: i16,
    pub logo_url: Option<String>,
    pub profile: Option<BrandProfileDetail>,
    pub social_handles: Vec<SocialHandleItem>,
    pub domains: Vec<String>,
    pub completeness: BrandCompletenessDetail,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct BrandProfileDetail {
    pub tagline: Option<String>,
    pub description: Option<String>,
    pub founded_year: Option<i16>,
    pub hq_city: Option<String>,
    pub hq_state: Option<String>,
    pub hq_country: String,
    pub parent_company: Option<String>,
    pub ceo_name: Option<String>,
    pub employee_count_approx: Option<i32>,
    pub total_funding_usd: Option<i64>,
    pub latest_valuation_usd: Option<i64>,
    pub funding_stage: Option<String>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct SocialHandleItem {
    pub platform: String,
    pub handle: String,
    pub profile_url: Option<String>,
    pub follower_count: Option<i32>,
    pub is_verified: Option<bool>,
}

#[derive(Debug, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub(in crate::api) struct BrandCompletenessDetail {
    pub score: i32,
    pub has_profile: bool,
    pub has_description: bool,
    pub has_tagline: bool,
    pub has_founded_year: bool,
    pub has_location: bool,
    pub has_social_handles: bool,
    pub has_domains: bool,
    pub has_signals: bool,
    pub has_funding: bool,
    pub has_lab_tests: bool,
    pub has_legal: bool,
    pub has_sponsorships: bool,
    pub has_distributors: bool,
    pub has_media: bool,
}

pub(in crate::api) async fn get_brand(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<BrandProfileResponse>>, ApiError> {
    let brand = resolve_brand(&state.pool, &slug, &req_id.0).await?;

    let profile = scbdb_db::get_brand_profile(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let social_handles = scbdb_db::list_brand_social_handles(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let domains = scbdb_db::list_brand_feed_urls(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let completeness = scbdb_db::get_brand_completeness(&state.pool, brand.id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let completeness_detail = completeness.map_or_else(
        || BrandCompletenessDetail {
            score: 0,
            has_profile: false,
            has_description: false,
            has_tagline: false,
            has_founded_year: false,
            has_location: false,
            has_social_handles: false,
            has_domains: false,
            has_signals: false,
            has_funding: false,
            has_lab_tests: false,
            has_legal: false,
            has_sponsorships: false,
            has_distributors: false,
            has_media: false,
        },
        |c| BrandCompletenessDetail {
            score: c.score,
            has_profile: c.has_profile,
            has_description: c.has_description,
            has_tagline: c.has_tagline,
            has_founded_year: c.has_founded_year,
            has_location: c.has_location,
            has_social_handles: c.has_social_handles,
            has_domains: c.has_domains,
            has_signals: c.has_signals,
            has_funding: c.has_funding,
            has_lab_tests: c.has_lab_tests,
            has_legal: c.has_legal,
            has_sponsorships: c.has_sponsorships,
            has_distributors: c.has_distributors,
            has_media: c.has_media,
        },
    );

    let profile_detail = profile.map(|p| BrandProfileDetail {
        tagline: p.tagline,
        description: p.description,
        founded_year: p.founded_year,
        hq_city: p.hq_city,
        hq_state: p.hq_state,
        hq_country: p.hq_country,
        parent_company: p.parent_company,
        ceo_name: p.ceo_name,
        employee_count_approx: p.employee_count_approx,
        total_funding_usd: p.total_funding_usd,
        latest_valuation_usd: p.latest_valuation_usd,
        funding_stage: p.funding_stage,
    });

    let social_items = social_handles
        .into_iter()
        .map(|h| SocialHandleItem {
            platform: h.platform,
            handle: h.handle,
            profile_url: h.profile_url,
            follower_count: h.follower_count,
            is_verified: h.is_verified,
        })
        .collect();

    let data = BrandProfileResponse {
        id: brand.id,
        slug: brand.slug,
        name: brand.name,
        relationship: brand.relationship,
        tier: brand.tier,
        logo_url: brand.logo_url,
        profile: profile_detail,
        social_handles: social_items,
        domains,
        completeness: completeness_detail,
    };

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}
