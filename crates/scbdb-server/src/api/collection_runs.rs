use axum::{
    extract::{Query, State},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::RequestId;

use super::{map_db_error, normalize_limit, ApiError, ApiResponse, AppState, ResponseMeta};

#[derive(Debug, Deserialize)]
pub(super) struct CollectionRunsQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(super) struct CollectionRunItem {
    collection_run_id: Uuid,
    run_type: String,
    trigger_source: String,
    status: String,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    records_processed: i32,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
}

pub(super) async fn list_collection_runs(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Query(query): Query<CollectionRunsQuery>,
) -> Result<Json<ApiResponse<Vec<CollectionRunItem>>>, ApiError> {
    let rows = scbdb_db::list_collection_runs(&state.pool, normalize_limit(query.limit))
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let data = rows
        .into_iter()
        .map(|row| CollectionRunItem {
            collection_run_id: row.public_id,
            run_type: row.run_type,
            trigger_source: row.trigger_source,
            status: row.status,
            started_at: row.started_at,
            completed_at: row.completed_at,
            records_processed: row.records_processed,
            error_message: row.error_message,
            created_at: row.created_at,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

#[cfg(test)]
mod tests {
    use super::CollectionRunItem;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn collection_run_item_is_serializable() {
        let item = CollectionRunItem {
            collection_run_id: Uuid::new_v4(),
            run_type: "price_snapshot".to_string(),
            trigger_source: "manual".to_string(),
            status: "succeeded".to_string(),
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            records_processed: 12,
            error_message: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&item).expect("serialize collection run");
        assert!(json.contains("\"run_type\":\"price_snapshot\""));
        assert!(json.contains("\"records_processed\":12"));
    }
}
