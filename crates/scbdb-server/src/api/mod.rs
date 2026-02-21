mod bills;
mod locations;
mod pricing;
mod products;
mod sentiment;

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

pub(super) fn map_db_error(request_id: String, error: &scbdb_db::DbError) -> ApiError {
    tracing::error!(error = %error, "database query failed");
    ApiError::new(request_id, "internal_error", "database query failed")
}

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
        .route(
            "/api/v1/sentiment/summary",
            get(sentiment::list_sentiment_summary),
        )
        .route(
            "/api/v1/sentiment/snapshots",
            get(sentiment::list_sentiment_snapshots),
        )
        .route(
            "/api/v1/locations/summary",
            get(locations::list_locations_summary),
        )
        .route(
            "/api/v1/locations/by-state",
            get(locations::list_locations_by_state),
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
    use super::locations::{LocationsByStateItem, LocationsDashboardItem};
    use super::sentiment::SentimentSummaryItem;
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use tower::ServiceExt;

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

    async fn seed_pricing_brand_with_logo(pool: &sqlx::PgPool, slug: &str, logo_url: &str) {
        let brand_id: i64 = sqlx::query_scalar(
            "INSERT INTO brands (name, slug, relationship, tier, shop_url, logo_url, is_active) \
             VALUES ($1, $2, 'competitor', 1, $3, $4, true) RETURNING id",
        )
        .bind(format!("Brand {slug}"))
        .bind(slug)
        .bind(format!("https://{slug}.example.com"))
        .bind(logo_url)
        .fetch_one(pool)
        .await
        .expect("insert brand");

        let product_id: i64 = sqlx::query_scalar(
            "INSERT INTO products (brand_id, source_platform, source_product_id, name, status, metadata) \
             VALUES ($1, 'shopify', $2, $3, 'active', '{}'::jsonb) RETURNING id",
        )
        .bind(brand_id)
        .bind(format!("{slug}-product-1"))
        .bind(format!("Product {slug}"))
        .fetch_one(pool)
        .await
        .expect("insert product");

        let variant_id: i64 = sqlx::query_scalar(
            "INSERT INTO product_variants (product_id, source_variant_id, title, is_default, is_available) \
             VALUES ($1, $2, 'Default', true, true) RETURNING id",
        )
        .bind(product_id)
        .bind(format!("{slug}-variant-1"))
        .fetch_one(pool)
        .await
        .expect("insert variant");

        sqlx::query(
            "INSERT INTO price_snapshots (variant_id, captured_at, currency_code, price) \
             VALUES ($1, NOW(), 'USD', 24.99)",
        )
        .bind(variant_id)
        .execute(pool)
        .await
        .expect("insert snapshot");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn pricing_summary_includes_brand_logo_url(pool: sqlx::PgPool) {
        seed_pricing_brand_with_logo(
            &pool,
            "logo-summary-test",
            "https://cdn.example.com/summary-logo.svg",
        )
        .await;

        let auth = crate::middleware::AuthState::from_env(true).expect("auth");
        let app = build_app(AppState { pool }, auth, default_rate_limit_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pricing/summary")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json parse");
        let row = json["data"]
            .as_array()
            .expect("data array")
            .iter()
            .find(|r| r["brand_slug"] == "logo-summary-test")
            .expect("summary row exists");
        assert_eq!(
            row["brand_logo_url"].as_str(),
            Some("https://cdn.example.com/summary-logo.svg")
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn pricing_snapshots_include_brand_logo_url(pool: sqlx::PgPool) {
        seed_pricing_brand_with_logo(
            &pool,
            "logo-snapshots-test",
            "https://cdn.example.com/snapshots-logo.png",
        )
        .await;

        let auth = crate::middleware::AuthState::from_env(true).expect("auth");
        let app = build_app(AppState { pool }, auth, default_rate_limit_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pricing/snapshots?brand_slug=logo-snapshots-test&limit=5")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json parse");
        let row = json["data"]
            .as_array()
            .expect("data array")
            .first()
            .expect("snapshot row");
        assert_eq!(
            row["brand_logo_url"].as_str(),
            Some("https://cdn.example.com/snapshots-logo.png")
        );
    }

    // -------------------------------------------------------------------------
    // Locations — serialization unit tests (no DB)
    // -------------------------------------------------------------------------

    #[test]
    fn locations_dashboard_item_is_serializable() {
        let item = LocationsDashboardItem {
            brand_name: "Cann".to_string(),
            brand_slug: "cann".to_string(),
            active_count: 42,
            new_this_week: 5,
            states_covered: 7,
            locator_source: Some("locally".to_string()),
            last_seen_at: Some(Utc::now()),
        };
        let json = serde_json::to_string(&item).expect("serialize LocationsDashboardItem");
        assert!(
            json.contains("\"brand_slug\":\"cann\""),
            "serialized JSON should contain brand_slug"
        );
        assert!(
            json.contains("\"active_count\":42"),
            "serialized JSON should contain active_count"
        );
    }

    #[test]
    fn locations_by_state_item_is_serializable() {
        let item = LocationsByStateItem {
            state: "TX".to_string(),
            brand_count: 3,
            location_count: 12,
        };
        let json = serde_json::to_string(&item).expect("serialize LocationsByStateItem");
        let round_tripped: serde_json::Value =
            serde_json::from_str(&json).expect("deserialize LocationsByStateItem");
        assert_eq!(
            round_tripped["state"].as_str(),
            Some("TX"),
            "state round-trip"
        );
        assert_eq!(
            round_tripped["location_count"].as_i64(),
            Some(12),
            "location_count round-trip"
        );
    }

    // -------------------------------------------------------------------------
    // Locations — route integration tests (with DB)
    // -------------------------------------------------------------------------

    /// Insert a minimal brand row for locations tests and return its id.
    async fn seed_location_brand(pool: &sqlx::PgPool, slug: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "INSERT INTO brands (name, slug, relationship, tier, shop_url, is_active) \
             VALUES ($1, $2, 'portfolio', 1, $3, true) RETURNING id",
        )
        .bind(format!("Brand {slug}"))
        .bind(slug)
        .bind(format!("https://{slug}.example.com"))
        .fetch_one(pool)
        .await
        .expect("seed_location_brand failed")
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn locations_summary_returns_ok(pool: sqlx::PgPool) {
        let brand_id = seed_location_brand(&pool, "loc-summary-brand").await;

        sqlx::query(
            "INSERT INTO store_locations \
             (brand_id, location_key, name, state, country, locator_source, raw_data) \
             VALUES ($1, 'loc-sum-key-1', 'Summary Store CA', 'CA', 'US', 'locally', '{}'::jsonb)",
        )
        .bind(brand_id)
        .execute(&pool)
        .await
        .expect("insert location");

        let auth = crate::middleware::AuthState::from_env(true).expect("auth");
        let app = build_app(AppState { pool }, auth, default_rate_limit_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/locations/summary")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json parse");
        let data = json["data"].as_array().expect("data array");
        assert_eq!(data.len(), 1, "expected 1 brand row");
        assert_eq!(
            data[0]["brand_slug"].as_str(),
            Some("loc-summary-brand"),
            "brand_slug mismatch"
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn locations_by_state_returns_ok(pool: sqlx::PgPool) {
        let brand_id = seed_location_brand(&pool, "loc-state-brand").await;

        for key in &["loc-state-key-1", "loc-state-key-2"] {
            sqlx::query(
                "INSERT INTO store_locations \
                 (brand_id, location_key, name, state, country, raw_data) \
                 VALUES ($1, $2, 'State Store TX', 'TX', 'US', '{}'::jsonb)",
            )
            .bind(brand_id)
            .bind(key)
            .execute(&pool)
            .await
            .expect("insert location");
        }

        let auth = crate::middleware::AuthState::from_env(true).expect("auth");
        let app = build_app(AppState { pool }, auth, default_rate_limit_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/locations/by-state")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json parse");
        let data = json["data"].as_array().expect("data array");
        let tx_row = data
            .iter()
            .find(|r| r["state"].as_str() == Some("TX"))
            .expect("TX row missing from response");
        assert_eq!(
            tx_row["location_count"].as_i64(),
            Some(2),
            "TX location_count should be 2"
        );
    }
}
