//! F3: GET /api/v1/brands/:slug/signals — cursor-paginated signal feed.

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::RequestId;

use super::super::{map_db_error, normalize_limit, ApiError, ApiResponse, AppState, ResponseMeta};
use super::resolve_brand;

#[derive(Debug, Deserialize)]
pub(in crate::api) struct SignalsQuery {
    #[serde(rename = "type")]
    pub signal_type: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct SignalItem {
    pub id: i64,
    pub public_id: Uuid,
    pub signal_type: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub collected_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct PaginatedSignals {
    pub items: Vec<SignalItem>,
    pub next_cursor: Option<i64>,
}

pub(in crate::api) async fn list_brand_signals(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
    Path(slug): Path<String>,
    Query(query): Query<SignalsQuery>,
) -> Result<Json<ApiResponse<PaginatedSignals>>, ApiError> {
    let brand = resolve_brand(&state.pool, &slug, &req_id.0).await?;
    let limit = normalize_limit(query.limit);

    let rows = scbdb_db::list_brand_signals(
        &state.pool,
        brand.id,
        query.signal_type.as_deref(),
        limit + 1, // fetch one extra to detect next page
        query.cursor,
    )
    .await
    .map_err(|e| map_db_error(req_id.0.clone(), &e))?;

    // `normalize_limit` clamps to 1..=200, so the conversion is always safe.
    let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
    let has_more = rows.len() > limit_usize;
    let take = if has_more { limit_usize } else { rows.len() };

    let items: Vec<SignalItem> = rows
        .into_iter()
        .take(take)
        .map(|r| SignalItem {
            id: r.id,
            public_id: r.public_id,
            signal_type: r.signal_type,
            title: r.title,
            summary: r.summary,
            source_url: r.source_url,
            image_url: r.image_url,
            published_at: r.published_at,
            collected_at: r.collected_at,
        })
        .collect();

    let next_cursor = if has_more {
        items.last().map(|item| item.id)
    } else {
        None
    };

    Ok(Json(ApiResponse {
        data: PaginatedSignals { items, next_cursor },
        meta: ResponseMeta::new(req_id.0),
    }))
}
