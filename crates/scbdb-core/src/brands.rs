use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ConfigError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Relationship {
    Portfolio,
    Competitor,
}

impl std::fmt::Display for Relationship {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Relationship::Portfolio => write!(f, "portfolio"),
            Relationship::Competitor => write!(f, "competitor"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandConfig {
    pub name: String,
    pub relationship: Relationship,
    pub tier: u8,
    pub domain: Option<String>,
    pub shop_url: Option<String>,
    pub notes: Option<String>,
}

impl BrandConfig {
    /// Generate a URL-safe slug from the brand name.
    #[must_use]
    pub fn slug(&self) -> String {
        self.name
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else if c == ' ' {
                    '-'
                } else {
                    '\0'
                }
            })
            .filter(|&c| c != '\0')
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[derive(Debug, Deserialize)]
pub struct BrandsFile {
    pub brands: Vec<BrandConfig>,
}

/// Load and validate the brands configuration from a YAML file.
///
/// # Errors
///
/// Returns `ConfigError` if the file cannot be read, parsed, or fails validation.
pub fn load_brands(path: &Path) -> Result<BrandsFile, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::BrandsFileIo {
        path: path.display().to_string(),
        source: e,
    })?;

    let brands_file: BrandsFile =
        serde_yaml::from_str(&content).map_err(ConfigError::BrandsFileParse)?;

    validate_brands(&brands_file)?;

    Ok(brands_file)
}

fn validate_brands(brands_file: &BrandsFile) -> Result<(), ConfigError> {
    let mut seen_names = HashSet::new();
    let mut seen_slugs = HashSet::new();

    for brand in &brands_file.brands {
        if brand.name.trim().is_empty() {
            return Err(ConfigError::Validation(
                "brand name must be non-empty".to_string(),
            ));
        }

        if ![1, 2, 3].contains(&brand.tier) {
            return Err(ConfigError::Validation(format!(
                "brand '{}' has invalid tier {}; must be 1, 2, or 3",
                brand.name, brand.tier
            )));
        }

        let lower_name = brand.name.to_lowercase();
        if !seen_names.insert(lower_name) {
            return Err(ConfigError::Validation(format!(
                "duplicate brand name: '{}'",
                brand.name
            )));
        }

        let slug = brand.slug();
        if !seen_slugs.insert(slug.clone()) {
            return Err(ConfigError::Validation(format!(
                "duplicate brand slug: '{}' (from brand '{}')",
                slug, brand.name
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
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
    }

    #[test]
    fn relationship_display() {
        assert_eq!(Relationship::Portfolio.to_string(), "portfolio");
        assert_eq!(Relationship::Competitor.to_string(), "competitor");
    }
}
