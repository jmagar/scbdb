//! HTTP client for Shopify's public `products.json` endpoint.

mod fetch_all;
mod origin;

use std::time::Duration;

use reqwest::Client;

use crate::error::ScraperError;
use crate::rate_limit::retry_with_backoff;
use crate::types::ShopifyProductsResponse;

pub use origin::extract_store_origin;
// Re-export for test visibility via `use super::*`
#[cfg(test)]
use origin::extract_domain;

/// Maximum number of pages to fetch before returning an error.
/// Prevents infinite loops on cycling cursors.
///
/// Note: each page request may be retried up to `max_retries` times on
/// transient errors, so the effective worst-case request count is
/// `MAX_PAGES * (1 + max_retries)`.
pub(super) const MAX_PAGES: usize = 200;

pub(super) const BROWSER_FALLBACK_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// HTTP client for Shopify's public `products.json` endpoint.
///
/// Handles rate limiting (429), not-found (404), and other non-2xx responses
/// as typed errors. Returns pagination cursors extracted from the `Link` header
/// for callers to drive multi-page fetches.
///
/// Transient errors (429, network failures) are automatically retried with
/// exponential backoff up to `max_retries` additional attempts.
pub struct ShopifyClient {
    pub(super) client: Client,
    /// Maximum number of retry attempts after the first failure.
    pub(super) max_retries: u32,
    /// Base delay in seconds for exponential backoff: `backoff_base_secs * 2^attempt`.
    pub(super) backoff_base_secs: u64,
}

impl ShopifyClient {
    /// Creates a `ShopifyClient` with configured timeout, `User-Agent`, and retry policy.
    ///
    /// `max_retries` is the number of additional attempts after the first failure for
    /// retriable errors (429, network errors). Set to `0` to disable retries.
    ///
    /// `backoff_base_secs` controls the base delay for exponential backoff:
    /// the wait before the n-th retry is `backoff_base_secs * 2^(n-1)` seconds.
    ///
    /// # Errors
    ///
    /// Returns [`ScraperError::Http`] if the underlying `reqwest::Client`
    /// cannot be constructed (e.g., invalid TLS config).
    pub fn new(
        timeout_secs: u64,
        user_agent: &str,
        max_retries: u32,
        backoff_base_secs: u64,
    ) -> Result<Self, ScraperError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(user_agent)
            .build()?;
        Ok(Self {
            client,
            max_retries,
            backoff_base_secs,
        })
    }

    /// Fetches one page of products from a Shopify store's public
    /// `products.json` endpoint, with automatic retry on transient errors.
    ///
    /// # Errors
    ///
    /// - [`ScraperError::RateLimited`] — HTTP 429 after all retries exhausted.
    /// - [`ScraperError::NotFound`] — HTTP 404 (not retried).
    /// - [`ScraperError::UnexpectedStatus`] — any other non-2xx status (5xx retried, 4xx not).
    /// - [`ScraperError::Http`] — network or TLS failure after all retries exhausted.
    /// - [`ScraperError::Deserialize`] — response body is not valid JSON (not retried).
    pub async fn fetch_products_page(
        &self,
        shop_url: &str,
        limit: u32,
        page_info: Option<&str>,
    ) -> Result<(ShopifyProductsResponse, Option<String>), ScraperError> {
        self.fetch_products_page_with_user_agent(shop_url, limit, page_info, None)
            .await
    }

    pub(super) async fn fetch_products_page_with_user_agent(
        &self,
        shop_url: &str,
        limit: u32,
        page_info: Option<&str>,
        user_agent_override: Option<&str>,
    ) -> Result<(ShopifyProductsResponse, Option<String>), ScraperError> {
        let url = Self::products_url(shop_url, limit, page_info)?;
        let max_retries = self.max_retries;
        let backoff_base_secs = self.backoff_base_secs;
        let referer = extract_store_origin(shop_url);
        let user_agent_override = user_agent_override.map(str::to_owned);

        retry_with_backoff(max_retries, backoff_base_secs, || {
            let url = url.clone();
            let shop_url = shop_url.to_owned();
            let referer = referer.clone();
            let user_agent_override = user_agent_override.clone();
            async move {
                let mut request = self
                    .client
                    .get(&url)
                    .header(
                        reqwest::header::ACCEPT,
                        "application/json,text/html;q=0.9,*/*;q=0.8",
                    )
                    .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
                    .header(reqwest::header::REFERER, &referer)
                    .header(reqwest::header::CACHE_CONTROL, "no-cache");

                if let Some(ua) = &user_agent_override {
                    request = request.header(reqwest::header::USER_AGENT, ua);
                }

                let response = request.send().await?;
                let status = response.status();

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    let retry_after_secs = response
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(60);

                    let domain = origin::extract_domain(&shop_url);
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
                let parsed =
                    serde_json::from_str::<ShopifyProductsResponse>(&body).map_err(|e| {
                        ScraperError::Deserialize {
                            context: format!("products page from {shop_url}"),
                            source: e,
                        }
                    })?;

                Ok((parsed, link_header))
            }
        })
        .await
    }

    /// Builds the `products.json` URL for the given shop, page size, and
    /// optional cursor.
    ///
    /// # Errors
    ///
    /// Returns [`ScraperError::InvalidShopUrl`] if the extracted origin cannot
    /// be parsed as a valid URL base.
    fn products_url(
        shop_url: &str,
        limit: u32,
        page_info: Option<&str>,
    ) -> Result<String, ScraperError> {
        let origin = extract_store_origin(shop_url);
        let base = format!("{origin}/products.json");
        let mut url = reqwest::Url::parse(&base).map_err(|e| ScraperError::InvalidShopUrl {
            shop_url: shop_url.to_owned(),
            reason: format!("origin \"{origin}\" is not a valid URL base: {e}"),
        })?;

        url.query_pairs_mut()
            .append_pair("limit", &limit.to_string());

        if let Some(cursor) = page_info {
            url.query_pairs_mut().append_pair("page_info", cursor);
        }

        Ok(url.to_string())
    }
}

#[cfg(test)]
#[path = "../client_test.rs"]
mod tests;
