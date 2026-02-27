use axum::{
    extract::{Query, State},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::middleware::RequestId;

use super::{map_db_error, normalize_limit, ApiError, ApiResponse, AppState, ResponseMeta};

#[derive(Debug, Serialize)]
pub(super) struct SentimentSummaryItem {
    pub brand_name: String,
    pub brand_slug: String,
    pub score: Decimal,
    pub signal_count: i32,
    pub captured_at: DateTime<Utc>,
    pub metadata: Value,
}

// TODO: consider merging with SentimentSummaryItem if these remain identical —
// both structs have the same five fields. Kept separate for now in case the
// snapshot view diverges (e.g. adds a `window` or `delta` column).
#[derive(Debug, Serialize)]
pub(super) struct SentimentSnapshotItem {
    pub brand_name: String,
    pub brand_slug: String,
    pub score: Decimal,
    pub signal_count: i32,
    pub captured_at: DateTime<Utc>,
    pub metadata: Value,
}

#[derive(Debug, Deserialize)]
pub(super) struct SentimentSnapshotsQuery {
    pub limit: Option<i64>,
}

pub(super) async fn list_sentiment_summary(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
) -> Result<Json<ApiResponse<Vec<SentimentSummaryItem>>>, ApiError> {
    let rows = scbdb_db::list_sentiment_summary(&state.pool)
        .await
        .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let data = rows
        .into_iter()
        .map(|row| SentimentSummaryItem {
            brand_name: row.brand_name,
            brand_slug: row.brand_slug,
            score: row.score,
            signal_count: row.signal_count,
            captured_at: row.captured_at,
            metadata: row.metadata,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}

pub(super) async fn list_sentiment_snapshots(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Query(query): Query<SentimentSnapshotsQuery>,
) -> Result<Json<ApiResponse<Vec<SentimentSnapshotItem>>>, ApiError> {
    let rows =
        scbdb_db::list_sentiment_snapshots_dashboard(&state.pool, normalize_limit(query.limit))
            .await
            .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    let data = rows
        .into_iter()
        .map(|row| SentimentSnapshotItem {
            brand_name: row.brand_name,
            brand_slug: row.brand_slug,
            score: row.score,
            signal_count: row.signal_count,
            captured_at: row.captured_at,
            metadata: row.metadata,
        })
        .collect();

    Ok(Json(ApiResponse {
        data,
        meta: ResponseMeta::new(req_id.0),
    }))
}
