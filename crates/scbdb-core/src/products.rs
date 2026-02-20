use serde::{Deserialize, Serialize};

/// A product scraped from a brand's storefront, normalized for storage and
/// comparison across brands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedProduct {
    /// Shopify numeric product ID, stored as a string to avoid precision loss.
    pub source_product_id: String,
    /// Platform the product was scraped from (e.g., `"shopify"`).
    pub source_platform: String,
    pub name: String,
    /// Raw HTML from Shopify's `body_html` field.
    pub description: Option<String>,
    pub product_type: Option<String>,
    /// Individual tags split from Shopify's comma-separated tag string.
    pub tags: Vec<String>,
    /// Shopify URL slug, e.g. `"hi-boy-blood-orange-5mg"`.
    pub handle: Option<String>,
    /// Shopify product status: `"active"`, `"archived"`, or `"draft"`.
    pub status: String,
    /// Canonical storefront URL, e.g. `"https://drinkhi.com/products/hi-boy-blood-orange-5mg"`.
    pub source_url: Option<String>,
    pub variants: Vec<NormalizedVariant>,
}

impl NormalizedProduct {
    /// Returns the total number of variants for this product.
    #[must_use]
    pub fn variant_count(&self) -> usize {
        self.variants.len()
    }

    /// Returns `true` if at least one variant is currently available for purchase.
    #[must_use]
    pub fn has_available_variants(&self) -> bool {
        self.variants.iter().any(|v| v.is_available)
    }

    /// Returns the default variant (position 1 in Shopify), if present.
    #[must_use]
    pub fn default_variant(&self) -> Option<&NormalizedVariant> {
        self.variants.iter().find(|v| v.is_default)
    }
}

/// A single purchasable variant of a [`NormalizedProduct`], e.g. a specific
/// can size and THC dose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedVariant {
    /// Shopify numeric variant ID, stored as a string to avoid precision loss.
    pub source_variant_id: String,
    pub sku: Option<String>,
    /// Shopify variant display title, e.g. `"12oz / 5mg THC"`.
    pub title: String,
    /// Price as a decimal string, exactly as Shopify returns it, e.g. `"12.99"`.
    pub price: String,
    /// Pre-sale comparison price, if set.
    pub compare_at_price: Option<String>,
    /// ISO 4217 currency code (e.g., `"USD"`).
    pub currency_code: String,
    /// Canonical URL to this variant's product page, if available.
    pub source_url: Option<String>,
    /// Whether the variant is currently in stock and purchasable.
    pub is_available: bool,
    /// `true` for the Shopify position-1 variant (the storefront default).
    pub is_default: bool,
    /// THC dose in milligrams, parsed from the variant title (e.g. `"5mg"` → `5.0`).
    ///
    /// Boundary note: this is a scrape-time `f64` convenience type. Persistence
    /// converts to `NUMERIC(8,2)` in the DB layer, so values are rounded to two
    /// decimal places at write time.
    pub dosage_mg: Option<f64>,
    /// CBD dose in milligrams, parsed from the variant title (e.g. `"2mg CBD"` → `2.0`).
    ///
    /// Boundary note: converted to `NUMERIC(8,2)` when persisted.
    pub cbd_mg: Option<f64>,
    /// Numeric container size, parsed from the variant title (e.g. `"12oz"` → `12.0`).
    ///
    /// Boundary note: converted to `NUMERIC(10,2)` when persisted.
    pub size_value: Option<f64>,
    /// Unit for `size_value`, parsed from the variant title (e.g. `"oz"`).
    pub size_unit: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_variant(id: &str, is_available: bool, is_default: bool) -> NormalizedVariant {
        NormalizedVariant {
            source_variant_id: id.to_string(),
            sku: None,
            title: "12oz / 5mg THC".to_string(),
            price: "12.99".to_string(),
            compare_at_price: None,
            currency_code: "USD".to_string(),
            source_url: None,
            is_available,
            is_default,
            dosage_mg: Some(5.0),
            cbd_mg: None,
            size_value: Some(12.0),
            size_unit: Some("oz".to_string()),
        }
    }

    fn make_product(variants: Vec<NormalizedVariant>) -> NormalizedProduct {
        NormalizedProduct {
            source_product_id: "123456789".to_string(),
            source_platform: "shopify".to_string(),
            name: "Hi Boy Blood Orange".to_string(),
            description: Some("<p>A refreshing THC beverage.</p>".to_string()),
            product_type: Some("Beverage".to_string()),
            tags: vec!["thc".to_string(), "beverage".to_string()],
            handle: Some("hi-boy-blood-orange-5mg".to_string()),
            status: "active".to_string(),
            source_url: Some("https://drinkhi.com/products/hi-boy-blood-orange-5mg".to_string()),
            variants,
        }
    }

    #[test]
    fn variant_count_zero_when_no_variants() {
        let product = make_product(vec![]);
        assert_eq!(product.variant_count(), 0);
    }

    #[test]
    fn variant_count_matches_variants_len() {
        let product = make_product(vec![
            make_variant("1", true, true),
            make_variant("2", false, false),
            make_variant("3", true, false),
        ]);
        assert_eq!(product.variant_count(), 3);
    }

    #[test]
    fn has_available_variants_false_when_no_variants() {
        let product = make_product(vec![]);
        assert!(!product.has_available_variants());
    }

    #[test]
    fn has_available_variants_false_when_all_unavailable() {
        let product = make_product(vec![
            make_variant("1", false, true),
            make_variant("2", false, false),
        ]);
        assert!(!product.has_available_variants());
    }

    #[test]
    fn has_available_variants_true_when_at_least_one_available() {
        let product = make_product(vec![
            make_variant("1", false, true),
            make_variant("2", true, false),
        ]);
        assert!(product.has_available_variants());
    }

    #[test]
    fn has_available_variants_true_when_all_available() {
        let product = make_product(vec![
            make_variant("1", true, true),
            make_variant("2", true, false),
        ]);
        assert!(product.has_available_variants());
    }

    #[test]
    fn default_variant_returns_none_when_no_variants() {
        let product = make_product(vec![]);
        assert!(product.default_variant().is_none());
    }

    #[test]
    fn default_variant_returns_none_when_none_is_default() {
        let product = make_product(vec![
            make_variant("1", true, false),
            make_variant("2", true, false),
        ]);
        assert!(product.default_variant().is_none());
    }

    #[test]
    fn default_variant_returns_the_is_default_variant() {
        let product = make_product(vec![
            make_variant("99", false, false),
            make_variant("42", true, true),
        ]);
        let default = product
            .default_variant()
            .expect("expected a default variant");
        assert_eq!(default.source_variant_id, "42");
    }

    #[test]
    fn normalized_product_construction_roundtrips_fields() {
        let variant = make_variant("7", true, true);
        let product = make_product(vec![variant]);

        assert_eq!(product.source_product_id, "123456789");
        assert_eq!(product.name, "Hi Boy Blood Orange");
        assert_eq!(product.status, "active");
        assert_eq!(product.tags, vec!["thc", "beverage"]);
        assert_eq!(product.handle.as_deref(), Some("hi-boy-blood-orange-5mg"));
    }

    #[test]
    fn normalized_variant_fields_are_accessible() {
        let variant = NormalizedVariant {
            source_variant_id: "999".to_string(),
            sku: Some("HI-BO-12-5".to_string()),
            title: "12oz / 5mg THC".to_string(),
            price: "12.99".to_string(),
            compare_at_price: Some("14.99".to_string()),
            currency_code: "USD".to_string(),
            source_url: Some("https://drinkhi.com/products/hi-boy".to_string()),
            is_available: true,
            is_default: true,
            dosage_mg: Some(5.0),
            cbd_mg: Some(2.0),
            size_value: Some(12.0),
            size_unit: Some("oz".to_string()),
        };

        assert_eq!(variant.source_variant_id, "999");
        assert_eq!(variant.sku.as_deref(), Some("HI-BO-12-5"));
        assert_eq!(variant.price, "12.99");
        assert_eq!(variant.compare_at_price.as_deref(), Some("14.99"));
        assert!(variant.is_available);
        assert!(variant.is_default);
        assert_eq!(variant.dosage_mg, Some(5.0));
        assert_eq!(variant.cbd_mg, Some(2.0));
        assert_eq!(variant.size_value, Some(12.0));
        assert_eq!(variant.size_unit.as_deref(), Some("oz"));
    }

    #[test]
    fn serde_roundtrip_product() {
        let product = make_product(vec![make_variant("1", true, true)]);
        let json = serde_json::to_string(&product).expect("serialization failed");
        let decoded: NormalizedProduct =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(decoded.source_product_id, product.source_product_id);
        assert_eq!(decoded.name, product.name);
        assert_eq!(decoded.variants.len(), 1);
        assert_eq!(
            decoded.variants[0].source_variant_id,
            product.variants[0].source_variant_id
        );
    }
}
