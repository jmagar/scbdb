mod bills;
mod pricing;
mod products;
pub mod sentiment;

use axum::{
    extract::State,
    http::{header, HeaderName, Method, StatusCode},
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

use crate::middleware::{
    enforce_rate_limit, request_id, require_bearer_auth, AuthState, RateLimitState, RequestId,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    pub meta: ResponseMeta,
}

#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: ErrorBody,
    pub meta: ResponseMeta,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HealthData {
    status: &'static str,
    database: &'static str,
}

impl ResponseMeta {
    pub(super) fn new(request_id: String) -> Self {
        Self {
            request_id,
            timestamp: Utc::now(),
        }
    }
}

impl ApiError {
    pub fn new(
        request_id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            error: ErrorBody {
                code: code.into(),
                message: message.into(),
            },
            meta: ResponseMeta::new(request_id.into()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.code.as_str() {
            "not_found" => StatusCode::NOT_FOUND,
            "unauthorized" => StatusCode::UNAUTHORIZED,
            "bad_request" | "validation_error" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

pub(super) fn normalize_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).clamp(1, 200)
}

pub(super) fn map_db_error(request_id: String, error: scbdb_db::DbError) -> ApiError {
    tracing::error!(error = %error, "database query failed");
    ApiError::new(request_id, "internal_error", "database query failed")
}

#[must_use]
pub fn build_app(state: AppState, auth: AuthState, rate_limit: RateLimitState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("x-request-id"),
        ]);

    let public_routes = Router::new().route("/api/v1/health", get(health));

    let protected_routes = Router::new()
        .route("/api/v1/products", get(products::list_products))
        .route(
            "/api/v1/pricing/snapshots",
            get(pricing::list_pricing_snapshots),
        )
        .route(
            "/api/v1/pricing/summary",
            get(pricing::list_pricing_summary),
        )
        .route("/api/v1/bills", get(bills::list_bills))
        .route(
            "/api/v1/bills/{bill_id}/events",
            get(bills::list_bill_events),
        )
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn_with_state(
                    rate_limit,
                    enforce_rate_limit,
                ))
                .layer(axum::middleware::from_fn_with_state(
                    auth,
                    require_bearer_auth,
                )),
        );

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(
            ServiceBuilder::new()
                .layer(cors)
                .layer(axum::middleware::from_fn(request_id)),
        )
        .with_state(state)
}

async fn health(
    State(state): State<AppState>,
    Extension(req_id): Extension<RequestId>,
) -> impl IntoResponse {
    let meta = ResponseMeta::new(req_id.0);

    match scbdb_db::health_check(&state.pool).await {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiResponse {
                data: HealthData {
                    status: "ok",
                    database: "ok",
                },
                meta,
            }),
        ),
        Err(e) => {
            tracing::warn!(error = %e, "health check: database unavailable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse {
                    data: HealthData {
                        status: "degraded",
                        database: "unavailable",
                    },
                    meta,
                }),
            )
        }
    }
}

pub fn default_rate_limit_state() -> RateLimitState {
    RateLimitState::new(120, Duration::from_secs(60))
}

#[cfg(test)]
mod tests {
    use super::sentiment::SentimentSummaryItem;
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;

    #[test]
    fn sentiment_summary_item_is_serializable() {
        // Proves the type compiles and serde works — no DB needed.
        let item = SentimentSummaryItem {
            brand_name: "Cann".to_string(),
            brand_slug: "cann".to_string(),
            score: Decimal::new(42, 2), // 0.42
            signal_count: 18,
            captured_at: Utc::now(),
        };
        let json = serde_json::to_string(&item).expect("serialize");
        assert!(json.contains("\"brand_slug\":\"cann\""));
    }

    #[test]
    fn normalize_limit_applies_defaults_and_bounds() {
        assert_eq!(normalize_limit(None), 50);
        assert_eq!(normalize_limit(Some(0)), 1);
        assert_eq!(normalize_limit(Some(1_000)), 200);
        assert_eq!(normalize_limit(Some(25)), 25);
    }

    #[test]
    fn api_error_validation_error_maps_to_bad_request() {
        let response = ApiError::new("req-1", "validation_error", "invalid input").into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
