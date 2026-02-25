mod bills;
mod brands;
mod locations;
mod pricing;
mod products;
mod sentiment;

use axum::{
    extract::State,
    http::{header, HeaderName, Method, StatusCode},
    response::IntoResponse,
    routing::{get, put},
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
            "conflict" => StatusCode::CONFLICT,
            "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
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

fn build_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("x-request-id"),
        ])
}

fn protected_router(auth: AuthState, rate_limit: RateLimitState) -> Router<AppState> {
    Router::new()
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
        .route("/api/v1/bills/{bill_id}/texts", get(bills::list_bill_texts))
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
        .route("/api/v1/locations/pins", get(locations::list_location_pins))
        .route(
            "/api/v1/brands",
            get(brands::list_brands).post(brands::create_brand),
        )
        .route(
            "/api/v1/brands/{slug}",
            get(brands::get_brand)
                .patch(brands::update_brand)
                .delete(brands::deactivate_brand),
        )
        .route(
            "/api/v1/brands/{slug}/signals",
            get(brands::list_brand_signals),
        )
        .route("/api/v1/brands/{slug}/funding", get(brands::list_funding))
        .route(
            "/api/v1/brands/{slug}/lab-tests",
            get(brands::list_lab_tests),
        )
        .route("/api/v1/brands/{slug}/legal", get(brands::list_legal))
        .route(
            "/api/v1/brands/{slug}/sponsorships",
            get(brands::list_sponsorships),
        )
        .route(
            "/api/v1/brands/{slug}/distributors",
            get(brands::list_distributors),
        )
        .route(
            "/api/v1/brands/{slug}/competitors",
            get(brands::list_competitors),
        )
        .route("/api/v1/brands/{slug}/media", get(brands::list_media))
        .route(
            "/api/v1/brands/{slug}/profile",
            put(brands::upsert_brand_profile),
        )
        .route(
            "/api/v1/brands/{slug}/social",
            put(brands::upsert_brand_social),
        )
        .route(
            "/api/v1/brands/{slug}/domains",
            put(brands::upsert_brand_domains),
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
        )
}

pub fn build_app(state: AppState, auth: AuthState, rate_limit: RateLimitState) -> Router {
    let public_routes = Router::new().route("/api/v1/health", get(health));

    Router::new()
        .merge(public_routes)
        .merge(protected_router(auth, rate_limit))
        .layer(
            ServiceBuilder::new()
                .layer(build_cors())
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
    use super::locations::{
        LocationPinItem, LocationsByStateItem, LocationsDashboardItem, PaginatedLocationPins,
    };
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
            metadata: serde_json::json!({}),
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
    fn location_pin_item_is_serializable() {
        let item = LocationPinItem {
            id: 42,
            latitude: 30.2672,
            longitude: -97.7431,
            store_name: "Pin Store".to_string(),
            address_line1: Some("123 Main St".to_string()),
            city: Some("Austin".to_string()),
            state: Some("TX".to_string()),
            zip: Some("78701".to_string()),
            locator_source: Some("locally".to_string()),
            brand_name: "Test Brand".to_string(),
            brand_slug: "test-brand".to_string(),
            brand_relationship: "portfolio".to_string(),
            brand_tier: 1,
        };
        let json = serde_json::to_string(&item).expect("serialize LocationPinItem");
        let round_tripped: serde_json::Value =
            serde_json::from_str(&json).expect("deserialize LocationPinItem");
        assert_eq!(
            round_tripped["brand_slug"].as_str(),
            Some("test-brand"),
            "brand_slug round-trip"
        );
        assert_eq!(
            round_tripped["brand_tier"].as_i64(),
            Some(1),
            "brand_tier round-trip"
        );
        assert!(
            (round_tripped["latitude"].as_f64().unwrap() - 30.2672).abs() < 0.001,
            "latitude round-trip"
        );
    }

    #[test]
    fn paginated_location_pins_includes_next_cursor() {
        let paginated = PaginatedLocationPins {
            items: vec![LocationPinItem {
                id: 99,
                latitude: 30.2672,
                longitude: -97.7431,
                store_name: "Cursor Store".to_string(),
                address_line1: None,
                city: None,
                state: None,
                zip: None,
                locator_source: None,
                brand_name: "Brand".to_string(),
                brand_slug: "brand".to_string(),
                brand_relationship: "portfolio".to_string(),
                brand_tier: 1,
            }],
            next_cursor: Some(99),
        };
        let json = serde_json::to_string(&paginated).expect("serialize PaginatedLocationPins");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed["next_cursor"].as_i64(), Some(99));
        assert_eq!(parsed["items"].as_array().map(|a| a.len()), Some(1));
    }

    #[test]
    fn paginated_location_pins_null_cursor_when_no_more() {
        let paginated = PaginatedLocationPins {
            items: vec![],
            next_cursor: None,
        };
        let json = serde_json::to_string(&paginated).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert!(parsed["next_cursor"].is_null());
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

    // -------------------------------------------------------------------------
    // Brands — integration tests (with DB)
    // -------------------------------------------------------------------------

    /// Seed a minimal brand for brand API tests.
    async fn seed_brand(pool: &sqlx::PgPool, slug: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "INSERT INTO brands (name, slug, relationship, tier, shop_url, is_active) \
             VALUES ($1, $2, 'competitor', 1, $3, true) RETURNING id",
        )
        .bind(format!("Brand {slug}"))
        .bind(slug)
        .bind(format!("https://{slug}.example.com"))
        .fetch_one(pool)
        .await
        .expect("seed_brand failed")
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn list_brands_returns_ok(pool: sqlx::PgPool) {
        seed_brand(&pool, "test-brand-list").await;

        let auth = crate::middleware::AuthState::from_env(true).expect("auth");
        let app = build_app(AppState { pool }, auth, default_rate_limit_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/brands")
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
        assert_eq!(data.len(), 1, "expected 1 brand");
        assert_eq!(data[0]["slug"].as_str(), Some("test-brand-list"));
        assert_eq!(data[0]["completeness_score"].as_i64(), Some(0));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn get_brand_returns_404_for_unknown_slug(pool: sqlx::PgPool) {
        let auth = crate::middleware::AuthState::from_env(true).expect("auth");
        let app = build_app(AppState { pool }, auth, default_rate_limit_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/brands/nonexistent-slug-xyz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn get_brand_returns_brand_profile(pool: sqlx::PgPool) {
        seed_brand(&pool, "test-brand-detail").await;

        let auth = crate::middleware::AuthState::from_env(true).expect("auth");
        let app = build_app(AppState { pool }, auth, default_rate_limit_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/brands/test-brand-detail")
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
        assert_eq!(json["data"]["slug"].as_str(), Some("test-brand-detail"));
        assert!(json["data"]["completeness"].is_object());
        assert!(json["data"]["social_handles"].is_array());
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
