//! Normalization from raw Shopify types to [`scbdb_core::NormalizedProduct`].
//!
//! Dosage and size parsing is delegated to [`crate::parse`]; this module
//! focuses on structural conversion from Shopify API shapes.

use scbdb_core::{NormalizedProduct, NormalizedVariant};

use crate::error::ScraperError;
use crate::parse::{parse_cbd_mg, parse_size, parse_thc_mg};
use crate::types::{ShopifyProduct, ShopifyVariant};

/// Normalizes a raw [`ShopifyProduct`] into a [`NormalizedProduct`].
///
/// # Errors
///
/// Returns [`ScraperError::Normalization`] if the product has no variants.
pub fn normalize_product(
    product: ShopifyProduct,
    shop_url: &str,
) -> Result<NormalizedProduct, ScraperError> {
    if product.variants.is_empty() {
        return Err(ScraperError::Normalization {
            source_product_id: product.id.to_string(),
            reason: "product has no variants".into(),
        });
    }

    let source_product_id = product.id.to_string();
    let source_url = Some(format!(
        "{}/products/{}",
        shop_url.trim_end_matches('/'),
        product.handle
    ));

    // Normalize product_type: treat empty string as absent.
    let product_type = product.product_type.filter(|s| !s.is_empty());

    // The position-1 variant is the storefront default. If no position data
    // exists, fall back to the first variant by index.
    let has_position_data = product.variants.iter().any(|v| v.position.is_some());

    let variants = product
        .variants
        .into_iter()
        .enumerate()
        .map(|(idx, variant)| {
            let is_default = if has_position_data {
                variant.position == Some(1)
            } else {
                idx == 0
            };
            normalize_variant(variant, is_default, &source_product_id)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(NormalizedProduct {
        source_product_id,
        source_platform: "shopify".to_string(),
        name: product.title,
        description: product.body_html,
        product_type,
        tags: product.tags,
        handle: Some(product.handle),
        status: product.status.unwrap_or_else(|| "active".to_string()),
        source_url,
        variants,
    })
}

/// Normalizes a raw [`ShopifyVariant`] into a [`NormalizedVariant`].
///
/// # Errors
///
/// Returns [`ScraperError::Normalization`] if required fields cannot be
/// interpreted (currently none — all failures are soft/optional).
fn normalize_variant(
    variant: ShopifyVariant,
    is_default: bool,
    source_product_id: &str,
) -> Result<NormalizedVariant, ScraperError> {
    // Validate price is non-empty; this is always set by Shopify but guard
    // defensively.
    if variant.price.is_empty() {
        return Err(ScraperError::Normalization {
            source_product_id: source_product_id.to_owned(),
            reason: format!("variant {} has empty price", variant.id),
        });
    }

    // Parse dosage and size from the title before moving it into the struct.
    let dosage_mg = parse_thc_mg(&variant.title);
    let cbd_mg = parse_cbd_mg(&variant.title);
    let (size_value, size_unit) = parse_size(&variant.title).unzip();

    // Normalize SKU: treat empty string as absent.
    let sku = variant.sku.filter(|s| !s.is_empty());

    // Shopify sends `compare_at_price` as `null` when no sale is active.
    // We confirm the field is already `None` in that case (no "0.00" normalization
    // needed based on observed live data from drinkcann.com and drinkbrez.com).
    let compare_at_price = variant.compare_at_price;

    Ok(NormalizedVariant {
        source_variant_id: variant.id.to_string(),
        sku,
        title: variant.title,
        price: variant.price,
        compare_at_price,
        // Shopify's products.json does not expose currency per-variant;
        // the store currency is USD for all brands in scope.
        currency_code: "USD".to_string(),
        // Variant-specific URLs are not provided by Shopify's product API;
        // the product-level source_url covers linking back to the storefront.
        source_url: None,
        is_available: variant.available,
        is_default,
        dosage_mg,
        cbd_mg,
        size_value,
        size_unit,
    })
}

#[cfg(test)]
mod tests {
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
        let product =
            make_shopify_product(vec![make_shopify_variant(1, "12oz / 5mg THC", Some(1))]);
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
        let mut product =
            make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
        product.status = None;
        let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
        assert_eq!(normalized.status, "active");
    }

    #[test]
    fn normalize_product_filters_empty_product_type() {
        let mut product =
            make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
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
        let mut product =
            make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
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
        let mut product =
            make_shopify_product(vec![make_shopify_variant(1, "Default Title", Some(1))]);
        product.variants[0].compare_at_price = Some("15.99".to_owned());
        let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
        assert_eq!(
            normalized.variants[0].compare_at_price.as_deref(),
            Some("15.99")
        );
    }

    #[test]
    fn normalize_variant_parses_dosage_from_title() {
        let product =
            make_shopify_product(vec![make_shopify_variant(1, "12oz / 5mg THC", Some(1))]);
        let normalized = normalize_product(product, "https://drinkhi.com").unwrap();
        assert_eq!(normalized.variants[0].dosage_mg, Some(5.0));
        assert_eq!(normalized.variants[0].size_value, Some(12.0));
        assert_eq!(normalized.variants[0].size_unit.as_deref(), Some("oz"));
    }
}
