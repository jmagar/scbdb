//! Shopify API response types for the public `products.json` endpoint.
//!
//! ## Observed shape from live Shopify stores (drinkcann.com, drinkbrez.com)
//!
//! ### Tags
//! Shopify returns tags as a **JSON array of strings**, NOT a comma-separated
//! string. Example: `["blood orange cardamom", "ginger lemongrass"]`.
//! The legacy Liquid API documented tags as a comma-separated string, but the
//! products.json endpoint returns an array. `#[serde(default)]` handles the
//! empty-array case from stores with no tags.
//!
//! ### `compare_at_price`
//! Explicitly `null` when the variant is not on sale (not omitted, not `"0.00"`).
//! When a sale price exists, the field is a numeric decimal string, e.g. `"162.00"`.
//! We model it as `Option<String>` and pass it through as-is.
//!
//! ### Dosage / THC / CBD
//! No structured dosage fields exist in the products.json response.
//! - Some brands (drinkhighrise.com) include explicit mg values in variant
//!   titles like `"12oz / 5mg THC"`.
//! - Others (drinkcann.com) use brand-tier names like `"Hi Boy"` / `"HI'ER BOY"`
//!   with no mg values anywhere in the structured data.
//! - Some stores (drinkbrez.com) embed dosage in `body_html` only
//!   (e.g., `"3mg micronized THC, 6mg CBD"`).
//! - Tags may hint at restrictions: `"restricted-state-5mg"` implies 5mg but
//!   is not a structured field.
//!   Parsing is done on a best-effort basis in `normalize.rs`.
//!
//! ### `product_type`
//! A plain string; may be empty (`""`), `"Beverages"`, or `"merch"`.
//! We model it as `Option<String>` and treat empty string as absent.
//!
//! ### `status`
//! Present in authenticated Admin API responses but may be absent from the
//! public `products.json` endpoint. We default to `"active"` when missing.
//!
//! ### `available` on variants
//! Boolean; `true` when the variant is in stock. May be absent on older stores.
//! We default to `true` (optimistic) when missing.
//!
//! ### `position` on variants
//! Integer; `1` for the storefront-default variant. Always present in observed
//! responses but we model as `Option<i32>` for safety.

use serde::Deserialize;

/// Top-level response from `GET /products.json`.
#[derive(Debug, Deserialize)]
pub struct ShopifyProductsResponse {
    pub products: Vec<ShopifyProduct>,
}

/// A single product from the Shopify storefront.
#[derive(Debug, Deserialize)]
pub struct ShopifyProduct {
    /// Shopify numeric product ID (e.g., `6789012345678`).
    pub id: i64,

    /// Display name of the product (e.g., `"Hi Boy Blood Orange 5mg"`).
    pub title: String,

    /// URL slug for the product page (e.g., `"hi-boy-blood-orange-5mg"`).
    pub handle: String,

    /// Raw HTML product description. May be `null` or absent.
    #[serde(default)]
    pub body_html: Option<String>,

    /// Product category string. May be empty string — normalized to `None`
    /// during normalization when empty.
    #[serde(default)]
    pub product_type: Option<String>,

    /// Tags as a JSON array of strings. Empty array `[]` when no tags.
    /// Observed format: `["blood orange cardamom", "ginger lemongrass"]`.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Publication status. May be absent from the public endpoint; defaults
    /// to `"active"` in normalization when missing.
    #[serde(default)]
    pub status: Option<String>,

    /// Vendor / brand name as configured in Shopify (e.g., `"CANN"`).
    #[serde(default)]
    pub vendor: Option<String>,

    /// Primary image object from Shopify.
    #[serde(default)]
    pub image: Option<ShopifyImage>,

    /// Full image gallery for the product.
    #[serde(default)]
    pub images: Vec<ShopifyImage>,

    /// All purchasable variants for this product.
    pub variants: Vec<ShopifyVariant>,
}

/// A single purchasable variant of a [`ShopifyProduct`].
#[derive(Debug, Deserialize)]
pub struct ShopifyVariant {
    /// Shopify numeric variant ID.
    pub id: i64,

    /// Display title of the variant. May be a size/dose string like
    /// `"12oz / 5mg THC"`, a pack name like `"Hi Boy"`, or `"Default Title"`.
    pub title: String,

    /// Stock-keeping unit. Present but may be an empty string on some stores.
    #[serde(default)]
    pub sku: Option<String>,

    /// Current price as a decimal string (e.g., `"30.00"`). Never null.
    pub price: String,

    /// Pre-sale / comparison price as a decimal string, or `null` when the
    /// variant is not on sale. Observed as `null` (not `"0.00"`) when absent.
    #[serde(default)]
    pub compare_at_price: Option<String>,

    /// Whether this variant is currently available for purchase.
    /// Defaults to `true` when absent (optimistic assumption).
    #[serde(default = "default_available")]
    pub available: bool,

    /// 1-based position; `1` is the storefront-default variant.
    #[serde(default)]
    pub position: Option<i32>,
}

/// A product image from Shopify `products.json`.
#[derive(Debug, Deserialize)]
pub struct ShopifyImage {
    /// Shopify numeric image ID.
    #[serde(default)]
    pub id: Option<i64>,
    /// Canonical CDN URL.
    pub src: String,
    /// Optional alt text.
    #[serde(default)]
    pub alt: Option<String>,
    /// 1-based image position.
    #[serde(default)]
    pub position: Option<i32>,
    /// Pixel width.
    #[serde(default)]
    pub width: Option<i32>,
    /// Pixel height.
    #[serde(default)]
    pub height: Option<i32>,
    /// Variant IDs associated with this image.
    #[serde(default)]
    pub variant_ids: Vec<i64>,
}

/// Default value for `ShopifyVariant::available` when the field is absent.
///
/// This cannot be a `const`: serde's `default = "...“` attribute expects a
/// function path to call for each missing field. `true` is intentional here
/// (we prefer optimistic availability when Shopify omits the field).
fn default_available() -> bool {
    true
}
