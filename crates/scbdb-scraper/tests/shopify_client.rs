//! Integration tests for `ShopifyClient::fetch_all_products`.
//!
//! Uses `wiremock` to stand up a local HTTP server for each test so no
//! real network traffic is made. Tests are grouped by scenario and cover
//! the happy paths (empty, single-page, multi-page) and every error
//! variant that `fetch_all_products` can propagate.

use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use scbdb_scraper::{ScraperError, ShopifyClient};

/// Builds a `ShopifyClient` suitable for tests: 5-second timeout, descriptive UA.
fn test_client() -> ShopifyClient {
    ShopifyClient::new(5, "scbdb-test/0.1").expect("failed to build test ShopifyClient")
}

/// Minimal valid one-product JSON fixture (id = 1).
fn one_product_json(id: i64) -> serde_json::Value {
    json!({
        "products": [{
            "id": id,
            "title": "Test Product",
            "handle": "test-product",
            "body_html": null,
            "product_type": null,
            "tags": [],
            "status": "active",
            "vendor": null,
            "variants": [{
                "id": 101,
                "title": "Default Title",
                "sku": null,
                "price": "12.99",
                "compare_at_price": null,
                "available": true,
                "position": 1
            }]
        }]
    })
}

// ---------------------------------------------------------------------------
// Test 1 – empty product list
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_all_products_returns_empty_vec_when_response_has_no_products() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/products.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({"products": []})))
        .mount(&server)
        .await;

    let client = test_client();
    let result = client.fetch_all_products(&server.uri(), 250, 0).await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    assert!(
        result.unwrap().is_empty(),
        "expected empty Vec when server returns no products"
    );
}

// ---------------------------------------------------------------------------
// Test 2 – single page with one product
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_all_products_returns_all_products_on_single_page() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/products.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&one_product_json(1)))
        .mount(&server)
        .await;

    let client = test_client();
    let result = client.fetch_all_products(&server.uri(), 250, 0).await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    let products = result.unwrap();
    assert_eq!(products.len(), 1, "expected exactly 1 product");
    assert_eq!(products[0].id, 1, "expected product id 1");
}

// ---------------------------------------------------------------------------
// Test 3 – pagination across multiple pages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_all_products_follows_pagination_across_multiple_pages() {
    let server = MockServer::start().await;

    // Page 1: returns product id=1 plus a Link header pointing to page 2.
    let next_link = format!(
        "<{base}/products.json?limit=250&page_info=cursor2>; rel=\"next\"",
        base = server.uri()
    );

    Mock::given(method("GET"))
        .and(path("/products.json"))
        // Match only requests WITHOUT a page_info query param (first page).
        .and(wiremock::matchers::query_param_is_missing("page_info"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&one_product_json(1))
                .insert_header("Link", next_link.as_str()),
        )
        .mount(&server)
        .await;

    // Page 2: returns product id=2, no Link header (last page).
    Mock::given(method("GET"))
        .and(path("/products.json"))
        .and(query_param("page_info", "cursor2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&one_product_json(2)))
        .mount(&server)
        .await;

    let client = test_client();
    let result = client.fetch_all_products(&server.uri(), 250, 0).await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    let products = result.unwrap();
    assert_eq!(products.len(), 2, "expected 2 products across 2 pages");
    assert_eq!(products[0].id, 1, "first product should have id 1");
    assert_eq!(products[1].id, 2, "second product should have id 2");
}

// ---------------------------------------------------------------------------
// Test 4 – 429 rate-limit propagation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_all_products_propagates_rate_limit_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/products.json"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "30"))
        .mount(&server)
        .await;

    let client = test_client();
    let result = client.fetch_all_products(&server.uri(), 250, 0).await;

    assert!(result.is_err(), "expected Err for 429 response");
    match result.unwrap_err() {
        ScraperError::RateLimited {
            retry_after_secs, ..
        } => {
            assert_eq!(
                retry_after_secs, 30,
                "retry_after_secs should match Retry-After header"
            );
        }
        other => panic!("expected ScraperError::RateLimited, got: {other:?}"),
    }
}

#[tokio::test]
async fn fetch_all_products_rate_limit_without_retry_after_defaults_to_60s() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/products.json"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let client = test_client();
    let result = client.fetch_all_products(&server.uri(), 250, 0).await;

    assert!(result.is_err(), "expected Err for 429 response");
    match result.unwrap_err() {
        ScraperError::RateLimited {
            retry_after_secs, ..
        } => {
            assert_eq!(retry_after_secs, 60, "expected default Retry-After of 60s");
        }
        other => panic!("expected ScraperError::RateLimited, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 5 – 404 not-found propagation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_all_products_propagates_not_found_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/products.json"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = test_client();
    let result = client.fetch_all_products(&server.uri(), 250, 0).await;

    assert!(result.is_err(), "expected Err for 404 response");
    assert!(
        matches!(result.unwrap_err(), ScraperError::NotFound { .. }),
        "expected ScraperError::NotFound"
    );
}

#[tokio::test]
async fn fetch_all_products_propagates_unexpected_status_error_for_5xx() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/products.json"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let client = test_client();
    let result = client.fetch_all_products(&server.uri(), 250, 0).await;

    assert!(result.is_err(), "expected Err for 503 response");
    match result.unwrap_err() {
        ScraperError::UnexpectedStatus { status, .. } => {
            assert_eq!(status, 503);
        }
        other => panic!("expected ScraperError::UnexpectedStatus, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 6 – malformed JSON propagation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_all_products_propagates_malformed_json_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/products.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not json"))
        .mount(&server)
        .await;

    let client = test_client();
    let result = client.fetch_all_products(&server.uri(), 250, 0).await;

    assert!(result.is_err(), "expected Err for malformed JSON response");
    assert!(
        matches!(result.unwrap_err(), ScraperError::Deserialize { .. }),
        "expected ScraperError::Deserialize"
    );
}
