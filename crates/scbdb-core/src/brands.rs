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
    #[serde(default)]
    pub store_locator_url: Option<String>,
    pub notes: Option<String>,
    /// Social platform handles: platform name → handle/username.
    /// e.g. `twitter: drinkcann`, `youtube: UCxxxxxxx`, `reddit: r/drinkcann`
    #[serde(default)]
    pub social: std::collections::HashMap<String, String>,
    /// All known domains for this brand (primary, redirects, defunct, etc.)
    #[serde(default)]
    pub domains: Vec<String>,
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
        if slug.is_empty() {
            return Err(ConfigError::Validation(format!(
                "brand '{}' produces an empty slug; use at least one ASCII letter or digit",
                brand.name
            )));
        }
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
#[path = "brands_test.rs"]
mod tests;
