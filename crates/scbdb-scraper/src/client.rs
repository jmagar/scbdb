use std::time::Duration;

use reqwest::Client;

use crate::error::ScraperError;
use crate::pagination::extract_next_cursor;
use crate::types::{ShopifyProduct, ShopifyProductsResponse};

/// HTTP client for Shopify's public `products.json` endpoint.
///
/// Handles rate limiting (429), not-found (404), and other non-2xx responses
/// as typed errors. Returns pagination cursors extracted from the `Link` header
/// for callers to drive multi-page fetches.
pub struct ShopifyClient {
    client: Client,
}

impl ShopifyClient {
    /// Creates a `ShopifyClient` with configured timeout and `User-Agent`.
    ///
    /// # Errors
    ///
    /// Returns [`ScraperError::Http`] if the underlying `reqwest::Client`
    /// cannot be constructed (e.g., invalid TLS config).
    pub fn new(timeout_secs: u64, user_agent: &str) -> Result<Self, ScraperError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(user_agent)
            .build()?;
        Ok(Self { client })
    }

    /// Fetches one page of products from a Shopify store's public
    /// `products.json` endpoint.
    ///
    /// Returns the parsed [`ShopifyProductsResponse`] and the raw value of the
    /// `Link` response header (if present). Callers should pass the `Link`
    /// value to [`crate::pagination::extract_next_cursor`] to determine
    /// whether additional pages exist.
    ///
    /// # Errors
    ///
    /// - [`ScraperError::RateLimited`] — HTTP 429; `retry_after_secs` is read
    ///   from the `Retry-After` header or defaults to 60.
    /// - [`ScraperError::NotFound`] — HTTP 404.
    /// - [`ScraperError::UnexpectedStatus`] — any other non-2xx status.
    /// - [`ScraperError::Http`] — network or TLS failure.
    /// - [`ScraperError::Deserialize`] — response body is not valid JSON or
    ///   does not match the expected shape.
    pub async fn fetch_products_page(
        &self,
        shop_url: &str,
        limit: u32,
        page_info: Option<&str>,
    ) -> Result<(ShopifyProductsResponse, Option<String>), ScraperError> {
        let url = Self::products_url(shop_url, limit, page_info);

        let response = self.client.get(&url).send().await?;
        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after_secs = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);

            let domain = extract_domain(shop_url);
            return Err(ScraperError::RateLimited {
                domain,
                retry_after_secs,
            });
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(ScraperError::NotFound { url });
        }

        if !status.is_success() {
            return Err(ScraperError::UnexpectedStatus {
                status: status.as_u16(),
                url,
            });
        }

        // Extract the Link header before consuming the response body.
        let link_header = response
            .headers()
            .get(reqwest::header::LINK)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        let body = response.text().await?;
        let parsed = serde_json::from_str::<ShopifyProductsResponse>(&body).map_err(|e| {
            ScraperError::Deserialize {
                context: format!("products page from {shop_url}"),
                source: e,
            }
        })?;

        Ok((parsed, link_header))
    }

    /// Fetches all products from a Shopify store by iterating through all pages.
    ///
    /// Starts with the first page (no cursor), follows `Link` header cursors until
    /// no `rel="next"` link is present, and returns all products collected.
    ///
    /// `inter_request_delay_ms` is the delay in milliseconds between page requests
    /// (applied after every page except the first).
    ///
    /// # Errors
    ///
    /// Propagates any error from [`Self::fetch_products_page`].
    pub async fn fetch_all_products(
        &self,
        shop_url: &str,
        limit: u32,
        inter_request_delay_ms: u64,
    ) -> Result<Vec<ShopifyProduct>, ScraperError> {
        let mut all_products: Vec<ShopifyProduct> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut is_first_page = true;

        loop {
            if !is_first_page && inter_request_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(inter_request_delay_ms)).await;
            }
            is_first_page = false;

            let (response, link_header) = self
                .fetch_products_page(shop_url, limit, cursor.as_deref())
                .await?;

            all_products.extend(response.products);

            cursor = extract_next_cursor(link_header.as_deref());
            if cursor.is_none() {
                break;
            }
        }

        Ok(all_products)
    }

    /// Builds the `products.json` URL for the given shop, page size, and
    /// optional cursor.
    ///
    /// When `page_info` is `Some`, Shopify requires that only `limit` and
    /// `page_info` are present — other filters are ignored or rejected.
    fn products_url(shop_url: &str, limit: u32, page_info: Option<&str>) -> String {
        let base = shop_url.trim_end_matches('/');
        match page_info {
            Some(cursor) => format!("{base}/products.json?limit={limit}&page_info={cursor}"),
            None => format!("{base}/products.json?limit={limit}"),
        }
    }
}

/// Extracts the hostname from a shop URL for use in error messages.
///
/// Falls back to the full URL string if parsing fails.
fn extract_domain(shop_url: &str) -> String {
    // Avoid pulling in the `url` crate for this minor operation.
    // Strip scheme and take up to the first `/`.
    let without_scheme = shop_url
        .strip_prefix("https://")
        .or_else(|| shop_url.strip_prefix("http://"))
        .unwrap_or(shop_url);
    without_scheme
        .split('/')
        .next()
        .unwrap_or(shop_url)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn products_url_without_cursor() {
        let url = ShopifyClient::products_url("https://drinkcann.com", 250, None);
        assert_eq!(url, "https://drinkcann.com/products.json?limit=250");
    }

    #[test]
    fn products_url_with_cursor() {
        let url =
            ShopifyClient::products_url("https://drinkcann.com", 250, Some("eyJsYXN0X2lkIjo2fQ"));
        assert_eq!(
            url,
            "https://drinkcann.com/products.json?limit=250&page_info=eyJsYXN0X2lkIjo2fQ"
        );
    }

    #[test]
    fn products_url_strips_trailing_slash() {
        let url = ShopifyClient::products_url("https://drinkcann.com/", 50, None);
        assert_eq!(url, "https://drinkcann.com/products.json?limit=50");
    }

    #[test]
    fn extract_domain_strips_scheme() {
        assert_eq!(extract_domain("https://drinkcann.com"), "drinkcann.com");
        assert_eq!(
            extract_domain("http://shop.example.com"),
            "shop.example.com"
        );
    }

    #[test]
    fn extract_domain_handles_path() {
        assert_eq!(
            extract_domain("https://drinkcann.com/products"),
            "drinkcann.com"
        );
    }

    #[test]
    fn extract_domain_fallback_no_scheme() {
        assert_eq!(extract_domain("drinkcann.com"), "drinkcann.com");
    }
}
