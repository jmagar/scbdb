use super::*;

// -----------------------------------------------------------------------
// normalize_product
// -----------------------------------------------------------------------

fn make_shopify_variant(id: i64, title: &str, position: Option<i32>) -> ShopifyVariant {
    ShopifyVariant {
        id,
        title: title.to_owned(),
        sku: Some("SKU-001".to_owned()),
        price: "12.99".to_owned(),
        compare_at_price: None,
        available: true,
        position,
    }
}

fn make_shopify_product(variants: Vec<ShopifyVariant>) -> ShopifyProduct {
    ShopifyProduct {
        id: 123_456_789,
        title: "Hi Boy Blood Orange 5mg".to_owned(),
        handle: "hi-boy-blood-orange-5mg".to_owned(),
        body_html: Some("<p>Great beverage.</p>".to_owned()),
        product_type: Some("Beverages".to_owned()),
        tags: vec!["thc".to_owned(), "beverage".to_owned()],
        status: Some("active".to_owned()),
        vendor: Some("Hi".to_owned()),
        variants,
    }
}

#[test]
fn normalize_product_sets_source_product_id() {
    let product = make_shopify_product(vec![make_shopify_variant(1, "12oz / 5mg THC", Some(1))]);
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(normalized.source_product_id, "123456789");
}

#[test]
fn normalize_product_builds_source_url() {
    let product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(
        normalized.source_url.as_deref(),
        Some("https://drinkhi.com/products/hi-boy-blood-orange-5mg")
    );
}

#[test]
fn normalize_product_builds_source_url_from_collection_path() {
    let product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    let normalized = normalize_product(product, "https://drinkhi.com/collections/all").unwrap();
    assert_eq!(
        normalized.source_url.as_deref(),
        Some("https://drinkhi.com/products/hi-boy-blood-orange-5mg")
    );
}

#[test]
fn normalize_product_strips_trailing_slash_from_shop_url() {
    let product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    let normalized = normalize_product(product, "https://drinkhi.com/").unwrap();
    assert_eq!(
        normalized.source_url.as_deref(),
        Some("https://drinkhi.com/products/hi-boy-blood-orange-5mg")
    );
}

#[test]
fn normalize_product_defaults_status_when_absent() {
    let mut product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    product.status = None;
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(normalized.status, "active");
}

#[test]
fn normalize_product_filters_empty_product_type() {
    let mut product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    product.product_type = Some(String::new());
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert!(normalized.product_type.is_none());
}

#[test]
fn normalize_product_error_when_no_variants() {
    let product = make_shopify_product(vec![]);
    let err = normalize_product(product, "https://drinkhi.com").unwrap_err();
    assert!(
        matches!(err, ScraperError::Normalization { reason, .. } if reason.contains("no variants"))
    );
}

#[test]
fn normalize_product_default_variant_by_position() {
    let variants = vec![
        make_shopify_variant(10, "Variant A", Some(2)),
        make_shopify_variant(11, "Variant B", Some(1)),
    ];
    let product = make_shopify_product(variants);
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    let default_v = normalized
        .variants
        .iter()
        .find(|v| v.is_default)
        .expect("expected a default variant");
    assert_eq!(default_v.source_variant_id, "11");
}

#[test]
fn normalize_product_default_variant_falls_back_to_first_when_no_position() {
    let variants = vec![
        make_shopify_variant(20, "First", None),
        make_shopify_variant(21, "Second", None),
    ];
    let product = make_shopify_product(variants);
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    let defaults: Vec<_> = normalized
        .variants
        .iter()
        .filter(|v| v.is_default)
        .collect();
    assert_eq!(defaults.len(), 1);
    assert_eq!(defaults[0].source_variant_id, "20");
}

#[test]
fn normalize_variant_empty_sku_becomes_none() {
    let mut product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    product.variants[0].sku = Some(String::new());
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert!(normalized.variants[0].sku.is_none());
}

#[test]
fn normalize_variant_compare_at_price_null_stays_none() {
    let product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert!(normalized.variants[0].compare_at_price.is_none());
}

#[test]
fn normalize_variant_compare_at_price_preserved_when_set() {
    let mut product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    product.variants[0].compare_at_price = Some("15.99".to_owned());
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(
        normalized.variants[0].compare_at_price.as_deref(),
        Some("15.99")
    );
}

#[test]
fn normalize_variant_parses_dosage_from_title() {
    let product = make_shopify_product(vec![make_shopify_variant(1, "12oz / 5mg THC", Some(1))]);
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(normalized.variants[0].dosage_mg, Some(5.0));
    assert_eq!(normalized.variants[0].size_value, Some(12.0));
    assert_eq!(normalized.variants[0].size_unit.as_deref(), Some("oz"));
}
