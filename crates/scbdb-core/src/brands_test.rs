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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
            notes: None,
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
            notes: None,
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
                notes: None,
            },
            BrandConfig {
                name: "cann".to_string(),
                relationship: Relationship::Competitor,
                tier: 2,
                domain: None,
                shop_url: None,
                notes: None,
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
                notes: None,
            },
            BrandConfig {
                name: "High--Rise".to_string(),
                relationship: Relationship::Competitor,
                tier: 1,
                domain: None,
                shop_url: None,
                notes: None,
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
                notes: None,
            },
            BrandConfig {
                name: "Cann".to_string(),
                relationship: Relationship::Competitor,
                tier: 1,
                domain: None,
                shop_url: None,
                notes: None,
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
            notes: None,
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
    let result = load_brands(&path);
    assert!(result.is_ok(), "failed to load brands.yaml: {result:?}");
    let brands_file = result.unwrap();
    assert!(!brands_file.brands.is_empty());
    let high_rise = brands_file
        .brands
        .iter()
        .find(|b| b.name == "High Rise")
        .expect("expected High Rise in brands.yaml");
    assert_eq!(high_rise.tier, 2);
    assert_eq!(high_rise.relationship, Relationship::Portfolio);
    assert_eq!(high_rise.slug(), "high-rise");

    let cann = brands_file
        .brands
        .iter()
        .find(|b| b.name == "Cann")
        .expect("expected Cann in brands.yaml");
    assert_eq!(cann.tier, 1);
    assert_eq!(cann.relationship, Relationship::Competitor);
    assert_eq!(cann.slug(), "cann");
}

#[test]
fn relationship_display() {
    assert_eq!(Relationship::Portfolio.to_string(), "portfolio");
    assert_eq!(Relationship::Competitor.to_string(), "competitor");
}
