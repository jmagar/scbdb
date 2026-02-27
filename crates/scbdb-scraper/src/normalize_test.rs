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
        image: None,
        images: vec![],
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

#[test]
fn normalize_product_preserves_vendor_from_shopify() {
    // make_shopify_product sets vendor: Some("Hi") — assert it flows through.
    let product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(normalized.vendor.as_deref(), Some("Hi"));
}

#[test]
fn normalize_product_maps_full_image_gallery() {
    let mut product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    product.image = Some(crate::types::ShopifyImage {
        id: Some(1001),
        src: "https://cdn.shopify.com/image-primary.jpg".to_owned(),
        alt: Some("primary".to_owned()),
        position: Some(1),
        width: Some(1200),
        height: Some(1200),
        variant_ids: vec![1],
    });
    product.images = vec![
        crate::types::ShopifyImage {
            id: Some(1001),
            src: "https://cdn.shopify.com/image-primary.jpg".to_owned(),
            alt: Some("primary".to_owned()),
            position: Some(1),
            width: Some(1200),
            height: Some(1200),
            variant_ids: vec![1],
        },
        crate::types::ShopifyImage {
            id: Some(1002),
            src: "https://cdn.shopify.com/image-secondary.jpg".to_owned(),
            alt: None,
            position: Some(2),
            width: Some(1200),
            height: Some(1200),
            variant_ids: vec![],
        },
    ];

    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(
        normalized.primary_image_url.as_deref(),
        Some("https://cdn.shopify.com/image-primary.jpg")
    );
    assert_eq!(normalized.image_gallery.len(), 2);
    assert_eq!(
        normalized.image_gallery[0].source_image_id.as_deref(),
        Some("1001")
    );
    assert_eq!(
        normalized.image_gallery[0].variant_source_ids,
        vec!["1".to_string()]
    );
}

#[test]
fn normalize_product_primary_image_prefers_default_variant_mapping() {
    let mut product = make_shopify_product(vec![
        make_shopify_variant(11, "Option A", Some(2)),
        make_shopify_variant(22, "Option B", Some(1)),
    ]);
    product.image = Some(crate::types::ShopifyImage {
        id: Some(2000),
        src: "https://cdn.shopify.com/fallback-primary.jpg".to_owned(),
        alt: None,
        position: Some(99),
        width: None,
        height: None,
        variant_ids: vec![],
    });
    product.images = vec![
        crate::types::ShopifyImage {
            id: Some(2001),
            src: "https://cdn.shopify.com/for-variant-11.jpg".to_owned(),
            alt: None,
            position: Some(1),
            width: None,
            height: None,
            variant_ids: vec![11],
        },
        crate::types::ShopifyImage {
            id: Some(2002),
            src: "https://cdn.shopify.com/for-variant-22.jpg".to_owned(),
            alt: None,
            position: Some(2),
            width: None,
            height: None,
            variant_ids: vec![22],
        },
    ];

    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(
        normalized.primary_image_url.as_deref(),
        Some("https://cdn.shopify.com/for-variant-22.jpg")
    );
}

#[test]
fn normalize_product_primary_image_falls_back_to_position_one() {
    let mut product = make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
    product.image = None;
    product.images = vec![
        crate::types::ShopifyImage {
            id: Some(3002),
            src: "https://cdn.shopify.com/position-2.jpg".to_owned(),
            alt: None,
            position: Some(2),
            width: None,
            height: None,
            variant_ids: vec![],
        },
        crate::types::ShopifyImage {
            id: Some(3001),
            src: "https://cdn.shopify.com/position-1.jpg".to_owned(),
            alt: None,
            position: Some(1),
            width: None,
            height: None,
            variant_ids: vec![],
        },
    ];

    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(
        normalized.primary_image_url.as_deref(),
        Some("https://cdn.shopify.com/position-1.jpg")
    );
}

#[test]
fn normalize_product_title_dosage_fallback_when_no_body_html() {
    // Better Than Booze pattern: dosage is in the product title ("2MG THC + 6MG CBD
    // Lemon Drop Martini"), variant titles are pack counts ("12-Pack"). When
    // body_html is absent, the title fallback should supply dosage_mg.
    let mut product = make_shopify_product(vec![make_shopify_variant(1, "12-Pack", Some(1))]);
    product.title = "2MG THC + 6MG CBD Lemon Drop Martini".to_owned();
    product.body_html = None; // no HTML to fall back to
    let normalized = normalize_product(product, "https://drinkbetterthanbooze.com").unwrap();
    assert_eq!(
        normalized.variants[0].dosage_mg,
        Some(2.0),
        "dosage should be extracted from product title when body_html is absent"
    );
}

#[test]
fn normalize_html_dosage_fallback_applies_uniformly_to_all_variants() {
    // When variant titles are bare names with no mg values, the dosage
    // extracted from body_html is applied to every variant. This is correct
    // for single-dose products (like BREZ) but is a known limitation for
    // multi-dose products — see normalize_variant doc for details.
    let mut product = make_shopify_product(vec![
        make_shopify_variant(1, "Hi Boy", Some(1)),
        make_shopify_variant(2, "Hi'er Boy", Some(2)),
    ]);
    product.body_html = Some("<p>3mg micronized THC per can</p>".to_owned());
    let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
    assert_eq!(
        normalized.variants[0].dosage_mg,
        Some(3.0),
        "first variant should receive html_dosage_fallback"
    );
    assert_eq!(
        normalized.variants[1].dosage_mg,
        Some(3.0),
        "second variant also receives the same html_dosage_fallback — uniform behavior"
    );
}
