//! Normalization from raw Shopify types to [`scbdb_core::NormalizedProduct`].
//!
//! Dosage and size parsing is delegated to [`crate::parse`]; this module
//! focuses on structural conversion from Shopify API shapes.

use scbdb_core::{NormalizedProduct, NormalizedVariant};

use crate::client::extract_store_origin;
use crate::error::ScraperError;
use crate::parse::{parse_cbd_mg, parse_dosage_from_html, parse_size, parse_thc_mg};
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
    let origin = extract_store_origin(shop_url);
    let source_url = Some(format!(
        "{}/products/{}",
        origin.trim_end_matches('/'),
        product.handle
    ));

    // Normalize product_type: treat empty string as absent.
    let product_type = product.product_type.filter(|s| !s.is_empty());

    // Best-effort dosage fallback from body_html for brands where variant
    // titles lack mg values (e.g., BREZ: "3mg micronized THC, 6mg CBD").
    let html_dosage_fallback: Option<f64> = product
        .body_html
        .as_deref()
        .and_then(parse_dosage_from_html);

    // The position-1 variant is the storefront default. If no position data
    // exists, or if position data exists but no variant claims position 1
    // (e.g., a variant was deleted), fall back to the first variant by index.
    let has_position_data = product.variants.iter().any(|v| v.position.is_some());
    let has_position_one = product.variants.iter().any(|v| v.position == Some(1));
    let use_position = has_position_data && has_position_one;

    let variants = product
        .variants
        .into_iter()
        .enumerate()
        .map(|(idx, variant)| {
            let is_default = if use_position {
                variant.position == Some(1)
            } else {
                idx == 0
            };
            normalize_variant(
                variant,
                is_default,
                &source_product_id,
                html_dosage_fallback,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let default_count = variants.iter().filter(|v| v.is_default).count();
    if default_count != 1 {
        tracing::warn!(
            source_product_id = %source_product_id,
            default_count,
            "expected exactly 1 default variant, got {}",
            default_count
        );
    }

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
        vendor: product.vendor,
        variants,
    })
}

/// Normalizes a raw [`ShopifyVariant`] into a [`NormalizedVariant`].
///
/// The `html_dosage_fallback` is the dosage extracted from the parent
/// product's `body_html` (computed once in [`normalize_product`]). It is
/// applied uniformly to **all** variants of the product when a variant's
/// title yields no dosage value.
///
/// **Limitation:** this fallback assumes every variant of a product has the
/// same dosage — which holds for single-dose brands like BREZ but will
/// produce incorrect data for products that offer multiple dosage strengths
/// across variants with bare titles (e.g., "Hi Boy" / "Hi'er Boy" paired
/// with `body_html` that mentions both 5mg and 10mg). For such products the
/// first parseable THC value in the HTML will be attributed to all variants.
///
/// # Errors
///
/// Returns [`ScraperError::Normalization`] if `variant.price` is empty or
/// not parseable as a number.
fn normalize_variant(
    variant: ShopifyVariant,
    is_default: bool,
    source_product_id: &str,
    html_dosage_fallback: Option<f64>,
) -> Result<NormalizedVariant, ScraperError> {
    // Validate price is non-empty and parseable as a numeric value.
    // Shopify always sets this field, but guard defensively — a malformed
    // value like "N/A" would otherwise fail silently at DB coercion time.
    if variant.price.is_empty() {
        return Err(ScraperError::Normalization {
            source_product_id: source_product_id.to_owned(),
            reason: format!("variant {} has empty price", variant.id),
        });
    }
    if variant.price.parse::<f64>().is_err() {
        return Err(ScraperError::Normalization {
            source_product_id: source_product_id.to_owned(),
            reason: format!(
                "variant {} has malformed price: {:?}",
                variant.id, variant.price
            ),
        });
    }

    // Parse dosage and size from the title before moving it into the struct.
    // Fall back to HTML body dosage when the variant title has no mg value.
    let dosage_mg = parse_thc_mg(&variant.title).or(html_dosage_fallback);
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
#[path = "normalize_test.rs"]
mod tests;
