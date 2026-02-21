//! URL origin and domain extraction utilities for the Shopify client.

/// Extracts the scheme+host origin from a shop URL.
///
/// Given `"https://drinkcann.com/collections/all"`, returns `"https://drinkcann.com"`.
/// This ensures `products.json` is always fetched from the store root, regardless
/// of whether the configured `shop_url` includes a collection path.
#[must_use]
pub fn extract_store_origin(shop_url: &str) -> String {
    reqwest::Url::parse(shop_url).map_or_else(
        |e| {
            tracing::warn!(
                shop_url,
                error = %e,
                "could not parse shop_url as URL — falling back to string split for origin extraction; check config/brands.yaml"
            );
            // fallback: take "https://host" by splitting on '/' and taking first 3 parts
            shop_url
                .trim_end_matches('/')
                .splitn(4, '/')
                .take(3)
                .collect::<Vec<_>>()
                .join("/")
        },
        |u| u.origin().ascii_serialization(),
    )
}

/// Extracts the hostname from a shop URL for use in error messages.
///
/// Falls back to the full URL string if parsing fails.
pub(super) fn extract_domain(shop_url: &str) -> String {
    reqwest::Url::parse(shop_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .unwrap_or_else(|| shop_url.to_owned())
}
