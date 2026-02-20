use std::time::Duration;

use reqwest::Client;

use crate::error::ScraperError;
use crate::pagination::extract_next_cursor;
use crate::rate_limit::retry_with_backoff;
use crate::types::{ShopifyProduct, ShopifyProductsResponse};

/// Maximum number of pages to fetch before returning an error.
/// Prevents infinite loops on cycling cursors.
const MAX_PAGES: usize = 200;

/// HTTP client for Shopify's public `products.json` endpoint.
///
/// Handles rate limiting (429), not-found (404), and other non-2xx responses
/// as typed errors. Returns pagination cursors extracted from the `Link` header
/// for callers to drive multi-page fetches.
///
/// Transient errors (429, network failures) are automatically retried with
/// exponential backoff up to `max_retries` additional attempts.
pub struct ShopifyClient {
    client: Client,
    /// Maximum number of retry attempts after the first failure.
    max_retries: u32,
    /// Base delay in seconds for exponential backoff: `backoff_base_secs * 2^attempt`.
    backoff_base_secs: u64,
}

/// Extracts the scheme+host origin from a shop URL.
///
/// Given `"https://drinkcann.com/collections/all"`, returns `"https://drinkcann.com"`.
/// This ensures `products.json` is always fetched from the store root, regardless
/// of whether the configured `shop_url` includes a collection path.
pub(crate) fn extract_store_origin(shop_url: &str) -> String {
    reqwest::Url::parse(shop_url).map_or_else(
        |_| {
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
    /// Retries up to `self.max_retries` times on [`ScraperError::RateLimited`]
    /// (HTTP 429) and [`ScraperError::Http`] (network failures), using
    /// exponential backoff with a base delay of `self.backoff_base_secs` seconds.
    ///
    /// Returns the parsed [`ShopifyProductsResponse`] and the raw value of the
    /// `Link` response header (if present). Callers should pass the `Link`
    /// value to [`crate::pagination::extract_next_cursor`] to determine
    /// whether additional pages exist.
    ///
    /// # Errors
    ///
    /// - [`ScraperError::RateLimited`] — HTTP 429 after all retries exhausted.
    /// - [`ScraperError::NotFound`] — HTTP 404 (not retried).
    /// - [`ScraperError::UnexpectedStatus`] — any other non-2xx status (not retried).
    /// - [`ScraperError::Http`] — network or TLS failure after all retries exhausted.
    /// - [`ScraperError::Deserialize`] — response body is not valid JSON or
    ///   does not match the expected shape (not retried).
    pub async fn fetch_products_page(
        &self,
        shop_url: &str,
        limit: u32,
        page_info: Option<&str>,
    ) -> Result<(ShopifyProductsResponse, Option<String>), ScraperError> {
        let url = Self::products_url(shop_url, limit, page_info);
        let max_retries = self.max_retries;
        let backoff_base_secs = self.backoff_base_secs;

        retry_with_backoff(max_retries, backoff_base_secs, || {
            let url = url.clone();
            let shop_url = shop_url.to_owned();
            async move {
                let response = self.client.get(&url).send().await?;
                let status = response.status();

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    let retry_after_secs = response
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(60);

                    let domain = extract_domain(&shop_url);
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
    /// Returns [`ScraperError::PaginationLimit`] if the number of pages exceeds
    /// [`MAX_PAGES`].
    pub async fn fetch_all_products(
        &self,
        shop_url: &str,
        limit: u32,
        inter_request_delay_ms: u64,
    ) -> Result<Vec<ShopifyProduct>, ScraperError> {
        let mut all_products: Vec<ShopifyProduct> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut is_first_page = true;
        let mut page_count = 0usize;

        loop {
            page_count += 1;
            if page_count > MAX_PAGES {
                return Err(ScraperError::PaginationLimit {
                    shop_url: shop_url.to_owned(),
                    max_pages: MAX_PAGES,
                });
            }

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
    /// Uses [`extract_store_origin`] to strip any collection path from the
    /// shop URL, ensuring we always hit `https://host/products.json`.
    ///
    /// When `page_info` is `Some`, the cursor is URL-encoded via `reqwest::Url`
    /// to avoid injection of unescaped characters.
    fn products_url(shop_url: &str, limit: u32, page_info: Option<&str>) -> String {
        let origin = extract_store_origin(shop_url);
        match page_info {
            Some(cursor) => {
                if let Ok(mut url) = reqwest::Url::parse(&format!("{origin}/products.json")) {
                    url.query_pairs_mut()
                        .append_pair("limit", &limit.to_string())
                        .append_pair("page_info", cursor);
                    url.to_string()
                } else {
                    // Fallback: build the URL manually if the origin is not
                    // parseable (e.g. no scheme). The cursor value comes from
                    // Shopify's own API and is base64-safe, so unencoded is
                    // acceptable as a last resort.
                    tracing::warn!(
                        shop_url,
                        "shop URL origin is not a valid URL base; using unencoded cursor"
                    );
                    format!("{origin}/products.json?limit={limit}&page_info={cursor}")
                }
            }
            None => format!("{origin}/products.json?limit={limit}"),
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
#[path = "client_test.rs"]
mod tests;
