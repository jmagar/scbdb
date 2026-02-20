use super::*;

#[test]
fn products_url_without_cursor() {
    let url =
        ShopifyClient::products_url("https://drinkcann.com/collections/all", 250, None).unwrap();
    assert_eq!(url, "https://drinkcann.com/products.json?limit=250");
}

#[test]
fn products_url_with_cursor() {
    let url = ShopifyClient::products_url(
        "https://drinkcann.com/collections/all",
        250,
        Some("eyJsYXN0X2lkIjo2fQ"),
    )
    .unwrap();
    assert_eq!(
        url,
        "https://drinkcann.com/products.json?limit=250&page_info=eyJsYXN0X2lkIjo2fQ"
    );
}

#[test]
fn products_url_strips_trailing_slash() {
    let url = ShopifyClient::products_url("https://drinkcann.com/", 50, None).unwrap();
    assert_eq!(url, "https://drinkcann.com/products.json?limit=50");
}

#[test]
fn products_url_bare_domain() {
    let url = ShopifyClient::products_url("https://drinkcann.com", 250, None).unwrap();
    assert_eq!(url, "https://drinkcann.com/products.json?limit=250");
}

#[test]
fn products_url_rejects_invalid_origin() {
    let result = ShopifyClient::products_url("not-a-url", 250, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ScraperError::InvalidShopUrl { .. }),
        "expected InvalidShopUrl, got: {err:?}"
    );
}

#[test]
fn extract_store_origin_strips_path() {
    assert_eq!(
        extract_store_origin("https://drinkcann.com/collections/all"),
        "https://drinkcann.com"
    );
}

#[test]
fn extract_store_origin_bare_domain() {
    assert_eq!(
        extract_store_origin("https://drinkcann.com"),
        "https://drinkcann.com"
    );
}

#[test]
fn extract_store_origin_trailing_slash() {
    assert_eq!(
        extract_store_origin("https://drinkcann.com/"),
        "https://drinkcann.com"
    );
}

#[test]
fn extract_domain_strips_scheme() {
    assert_eq!(extract_domain("https://drinkcann.com"), "drinkcann.com");
    assert_eq!(
        extract_domain("http://shop.example.com"),
        "shop.example.com"
    );
}

#[test]
fn extract_domain_handles_path() {
    assert_eq!(
        extract_domain("https://drinkcann.com/products"),
        "drinkcann.com"
    );
}

#[test]
fn extract_domain_fallback_no_scheme() {
    assert_eq!(extract_domain("drinkcann.com"), "drinkcann.com");
}
