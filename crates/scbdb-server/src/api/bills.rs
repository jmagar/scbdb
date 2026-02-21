use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::RequestId;

use super::{map_db_error, normalize_limit, ApiError, ApiResponse, AppState, ResponseMeta};

#[derive(Debug, Deserialize)]
pub(super) struct BillsQuery {
    pub jurisdiction: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(super) struct BillItem {
    bill_id: Uuid,
    jurisdiction: String,
    session: Option<String>,
    bill_number: String,
    title: String,
    status: String,
    status_date: Option<chrono::NaiveDate>,
    last_action_date: Option<chrono::NaiveDate>,
    source_url: Option<String>,
    event_count: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct BillEventItem {
    event_date: Option<chrono::NaiveDate>,
    event_type: Option<String>,
    chamber: Option<String>,
    description: String,
    source_url: Option<String>,
}

pub(super) async fn list_bills(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Query(query): Query<BillsQuery>,
) -> Result<Json<ApiResponse<Vec<BillItem>>>, ApiError> {
    let bills = scbdb_db::list_bills(
        &state.pool,
        query.jurisdiction.as_deref(),
        normalize_limit(query.limit),
    )
    .await
    .map_err(|e| map_db_error(req_id.0.clone(), e))?;

    let bill_ids: Vec<i64> = bills.iter().map(|bill| bill.id).collect();
    let events_by_bill = scbdb_db::list_bill_events_batch(&state.pool, &bill_ids)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), e))?;

    let data = bills
        .into_iter()
        .map(|bill| BillItem {
            bill_id: bill.public_id,
            jurisdiction: bill.jurisdiction,
            session: bill.session,
            bill_number: bill.bill_number,
            title: bill.title,
            status: bill.status,
            status_date: bill.status_date,
            last_action_date: bill.last_action_date,
            source_url: bill.source_url,
            event_count: events_by_bill.get(&bill.id).map_or(0, Vec::len),
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(super) async fn list_bill_events(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(bill_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<BillEventItem>>>, ApiError> {
    let bill = scbdb_db::get_bill_by_public_id(&state.pool, bill_id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), e))?;

    if bill.is_none() {
        return Err(ApiError::new(req_id.0, "not_found", "bill not found"));
    }

    let rows = scbdb_db::list_bill_events_by_public_id(&state.pool, bill_id)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), e))?;

    let data = rows
        .into_iter()
        .map(|event| BillEventItem {
            event_date: event.event_date,
            event_type: event.event_type,
            chamber: event.chamber,
            description: event.description,
            source_url: event.source_url,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}
