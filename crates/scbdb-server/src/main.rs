mod middleware;
mod scheduler;

use axum::{
    extract::State, http::StatusCode, response::IntoResponse, routing::get, Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::middleware::{request_id, RequestId};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

// -- API envelope types --

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

impl ResponseMeta {
    fn new(request_id: String) -> Self {
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

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.code.as_str() {
            "not_found" => axum::http::StatusCode::NOT_FOUND,
            "unauthorized" => axum::http::StatusCode::UNAUTHORIZED,
            "bad_request" => axum::http::StatusCode::BAD_REQUEST,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

// -- Health types --

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HealthData {
    status: &'static str,
    database: &'static str,
}

fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn(request_id))
                .layer(TraceLayer::new_for_http()),
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
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse {
                data: HealthData {
                    status: "degraded",
                    database: "unavailable",
                },
                meta,
            }),
        ),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let pool = scbdb_db::connect_pool_from_env().await?;
    scbdb_db::run_migrations(&pool).await?;

    let _scheduler = scheduler::build_scheduler().await?;

    let app = build_app(AppState { pool });

    let bind_addr = std::env::var("SCBDB_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let addr: SocketAddr = bind_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_data_serializes_correctly() {
        let data = HealthData {
            status: "ok",
            database: "connected",
        };
        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["database"], "connected");
    }

    #[test]
    fn api_response_envelope_serializes_correctly() {
        let meta = ResponseMeta {
            request_id: "test-id-123".to_string(),
            timestamp: DateTime::parse_from_rfc3339("2026-02-19T00:00:00Z")
                .expect("valid timestamp")
                .with_timezone(&Utc),
        };
        let response = ApiResponse {
            data: HealthData {
                status: "ok",
                database: "ok",
            },
            meta,
        };

        let json = serde_json::to_value(&response).expect("serializes");

        assert_eq!(json["data"]["status"], "ok");
        assert_eq!(json["data"]["database"], "ok");
        assert_eq!(json["meta"]["request_id"], "test-id-123");
        assert!(json["meta"]["timestamp"].is_string());
    }

    #[test]
    fn api_error_envelope_serializes_correctly() {
        let meta = ResponseMeta {
            request_id: "err-id-456".to_string(),
            timestamp: DateTime::parse_from_rfc3339("2026-02-19T00:00:00Z")
                .expect("valid timestamp")
                .with_timezone(&Utc),
        };
        let error = ApiError {
            error: ErrorBody {
                code: "validation_error".to_string(),
                message: "invalid input".to_string(),
            },
            meta,
        };

        let json = serde_json::to_value(&error).expect("serializes");

        assert_eq!(json["error"]["code"], "validation_error");
        assert_eq!(json["error"]["message"], "invalid input");
        assert_eq!(json["meta"]["request_id"], "err-id-456");
    }
}
