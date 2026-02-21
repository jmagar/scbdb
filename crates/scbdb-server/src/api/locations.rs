use axum::{extract::State, Extension, Json};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::middleware::RequestId;

use super::{map_db_error, ApiError, ApiResponse, AppState, ResponseMeta};

#[derive(Debug, Serialize)]
pub(super) struct LocationsDashboardItem {
    pub brand_name: String,
    pub brand_slug: String,
    pub active_count: i64,
    pub new_this_week: i64,
    pub states_covered: i64,
    pub locator_source: Option<String>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub(super) struct LocationsByStateItem {
    pub state: String,
    pub brand_count: i64,
    pub location_count: i64,
}

pub(super) async fn list_locations_summary(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
) -> Result<Json<ApiResponse<Vec<LocationsDashboardItem>>>, ApiError> {
    let rows = scbdb_db::list_locations_dashboard_summary(&state.pool)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &scbdb_db::DbError::from(e)))?;

    let data = rows
        .into_iter()
        .map(|row| LocationsDashboardItem {
            brand_name: row.brand_name,
            brand_slug: row.brand_slug,
            active_count: row.active_count,
            new_this_week: row.new_this_week,
            states_covered: row.states_covered,
            locator_source: row.locator_source,
            last_seen_at: row.last_seen_at,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

#[derive(Debug, Serialize)]
pub(super) struct LocationPinItem {
    pub latitude: f64,
    pub longitude: f64,
    pub store_name: String,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub locator_source: Option<String>,
    pub brand_name: String,
    pub brand_slug: String,
    pub brand_relationship: String,
    pub brand_tier: i16,
}

pub(super) async fn list_location_pins(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
) -> Result<Json<ApiResponse<Vec<LocationPinItem>>>, ApiError> {
    let rows = scbdb_db::list_active_location_pins(&state.pool)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &scbdb_db::DbError::from(e)))?;

    let data = rows
        .into_iter()
        .map(|row| LocationPinItem {
            latitude: row.latitude,
            longitude: row.longitude,
            store_name: row.store_name,
            address_line1: row.address_line1,
            city: row.city,
            state: row.state,
            zip: row.zip,
            locator_source: row.locator_source,
            brand_name: row.brand_name,
            brand_slug: row.brand_slug,
            brand_relationship: row.brand_relationship,
            brand_tier: row.brand_tier,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(super) async fn list_locations_by_state(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
) -> Result<Json<ApiResponse<Vec<LocationsByStateItem>>>, ApiError> {
    let rows = scbdb_db::list_locations_by_state(&state.pool)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &scbdb_db::DbError::from(e)))?;

    let data = rows
        .into_iter()
        .map(|row| LocationsByStateItem {
            state: row.state,
            brand_count: row.brand_count,
            location_count: row.location_count,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}
