use std::path::Path;

use super::*;

#[test]
fn slug_simple_name() {
    let brand = BrandConfig {
        name: "High Rise".to_string(),
        relationship: Relationship::Portfolio,
        tier: 2,
        domain: None,
        shop_url: None,
        store_locator_url: None,
        notes: None,
        social: Default::default(),
        domains: vec![],
    };
    assert_eq!(brand.slug(), "high-rise");
}

#[test]
fn slug_special_characters() {
    let brand = BrandConfig {
        name: "Uncle Arnie's".to_string(),
        relationship: Relationship::Competitor,
        tier: 1,
        domain: None,
        shop_url: None,
        store_locator_url: None,
        notes: None,
        social: Default::default(),
        domains: vec![],
    };
    assert_eq!(brand.slug(), "uncle-arnies");
}

#[test]
fn slug_accented_characters() {
    let brand = BrandConfig {
        name: "BRĒZ".to_string(),
        relationship: Relationship::Competitor,
        tier: 1,
        domain: None,
        shop_url: None,
        store_locator_url: None,
        notes: None,
        social: Default::default(),
        domains: vec![],
    };
    // Non-ASCII chars are stripped; no dash inserted between adjacent ASCII chars
    assert_eq!(brand.slug(), "brz");
}

#[test]
fn slug_with_tilde() {
    let brand = BrandConfig {
        name: "Señorita Drinks".to_string(),
        relationship: Relationship::Competitor,
        tier: 3,
        domain: None,
        shop_url: None,
        store_locator_url: None,
        notes: None,
        social: Default::default(),
        domains: vec![],
    };
    // ñ is non-ASCII and stripped; no dash between 'e' and 'o'
    assert_eq!(brand.slug(), "seorita-drinks");
}

#[test]
fn validate_rejects_invalid_tier() {
    let brands_file = BrandsFile {
        brands: vec![BrandConfig {
            name: "Test Brand".to_string(),
            relationship: Relationship::Competitor,
            tier: 5,
            domain: None,
            shop_url: None,
            store_locator_url: None,
            notes: None,
            social: Default::default(),
            domains: vec![],
        }],
    };
    let err = validate_brands(&brands_file).unwrap_err();
    assert!(err.to_string().contains("invalid tier 5"));
}

#[test]
fn validate_rejects_empty_name() {
    let brands_file = BrandsFile {
        brands: vec![BrandConfig {
            name: "  ".to_string(),
            relationship: Relationship::Competitor,
            tier: 1,
            domain: None,
            shop_url: None,
            store_locator_url: None,
            notes: None,
            social: Default::default(),
            domains: vec![],
        }],
    };
    let err = validate_brands(&brands_file).unwrap_err();
    assert!(err.to_string().contains("non-empty"));
}

#[test]
fn validate_rejects_duplicate_name() {
    let brands_file = BrandsFile {
        brands: vec![
            BrandConfig {
                name: "Cann".to_string(),
                relationship: Relationship::Competitor,
                tier: 1,
                domain: None,
                shop_url: None,
                store_locator_url: None,
                notes: None,
                social: Default::default(),
                domains: vec![],
            },
            BrandConfig {
                name: "cann".to_string(),
                relationship: Relationship::Competitor,
                tier: 2,
                domain: None,
                shop_url: None,
                store_locator_url: None,
                notes: None,
                social: Default::default(),
                domains: vec![],
            },
        ],
    };
    let err = validate_brands(&brands_file).unwrap_err();
    assert!(err.to_string().contains("duplicate brand name"));
}

#[test]
fn validate_rejects_duplicate_slug() {
    let brands_file = BrandsFile {
        brands: vec![
            BrandConfig {
                name: "High Rise".to_string(),
                relationship: Relationship::Portfolio,
                tier: 2,
                domain: None,
                shop_url: None,
                store_locator_url: None,
                notes: None,
                social: Default::default(),
                domains: vec![],
            },
            BrandConfig {
                name: "High--Rise".to_string(),
                relationship: Relationship::Competitor,
                tier: 1,
                domain: None,
                shop_url: None,
                store_locator_url: None,
                notes: None,
                social: Default::default(),
                domains: vec![],
            },
        ],
    };
    let err = validate_brands(&brands_file).unwrap_err();
    assert!(err.to_string().contains("duplicate brand"));
}

#[test]
fn validate_accepts_valid_brands() {
    let brands_file = BrandsFile {
        brands: vec![
            BrandConfig {
                name: "High Rise".to_string(),
                relationship: Relationship::Portfolio,
                tier: 2,
                domain: Some("highrisebev.com".to_string()),
                shop_url: None,
                store_locator_url: None,
                notes: None,
                social: Default::default(),
                domains: vec![],
            },
            BrandConfig {
                name: "Cann".to_string(),
                relationship: Relationship::Competitor,
                tier: 1,
                domain: None,
                shop_url: None,
                store_locator_url: None,
                notes: None,
                social: Default::default(),
                domains: vec![],
            },
        ],
    };
    assert!(validate_brands(&brands_file).is_ok());
}

#[test]
fn validate_rejects_empty_slug() {
    let brands_file = BrandsFile {
        brands: vec![BrandConfig {
            name: "---".to_string(),
            relationship: Relationship::Competitor,
            tier: 1,
            domain: None,
            shop_url: None,
            store_locator_url: None,
            notes: None,
            social: Default::default(),
            domains: vec![],
        }],
    };
    let err = validate_brands(&brands_file).unwrap_err();
    assert!(err.to_string().contains("empty slug"));
}

#[test]
fn load_brands_from_real_file() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config")
        .join("brands.yaml");
    assert!(
        path.exists(),
        "brands.yaml missing at {path:?} — required for this test"
    );
    let brands_file = load_brands(&path).expect("failed to load brands.yaml");
    assert!(
        !brands_file.brands.is_empty(),
        "brands.yaml should contain at least one brand"
    );
    for brand in &brands_file.brands {
        assert!(
            !brand.slug().is_empty(),
            "brand '{}' should have a non-empty slug",
            brand.name
        );
        assert!(
            !brand.name.trim().is_empty(),
            "all brands must have a non-empty name"
        );
    }
    // validate_brands is already called inside load_brands, so if we get
    // here the file passes all validation rules (unique names/slugs,
    // valid tiers, non-empty slugs).
}

#[test]
fn relationship_display() {
    assert_eq!(Relationship::Portfolio.to_string(), "portfolio");
    assert_eq!(Relationship::Competitor.to_string(), "competitor");
}

#[test]
fn brand_config_parses_social_and_domains() {
    let yaml = r#"
brands:
  - name: TestBrand
    relationship: competitor
    tier: 1
    social:
      twitter: testbrand
      youtube: UCxxxxxxxxxx
      reddit: r/testbrand
    domains:
      - testbrand.com
      - shop.testbrand.com
"#;
    let file: BrandsFile = serde_yaml::from_str(yaml).expect("parse");
    let brand = &file.brands[0];
    let social = &brand.social;
    assert_eq!(social.get("twitter").map(String::as_str), Some("testbrand"));
    assert_eq!(
        social.get("youtube").map(String::as_str),
        Some("UCxxxxxxxxxx")
    );
    assert_eq!(brand.domains.len(), 2);
    assert_eq!(brand.domains[0], "testbrand.com");
}
