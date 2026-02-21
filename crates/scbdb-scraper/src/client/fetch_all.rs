//! Multi-page product fetch loops for `ShopifyClient`.

use std::time::Duration;

use crate::error::ScraperError;
use crate::pagination::extract_next_cursor;
use crate::types::ShopifyProduct;

use super::ShopifyClient;
use super::MAX_PAGES;

impl ShopifyClient {
    /// Fetches all products from a Shopify store by iterating through all pages.
    ///
    /// Starts with the first page (no cursor), follows `Link` header cursors until
    /// no `rel="next"` link is present, and returns all products collected.
    ///
    /// `inter_request_delay_ms` is the delay in milliseconds between page requests
    /// (applied after every page except the first).
    ///
    /// **All-or-nothing semantics**: on any page failure (network error, rate limit,
    /// pagination limit), already-fetched products from earlier pages are discarded
    /// and the error is returned. This is intentional — partial product lists would
    /// produce incorrect deltas when compared against the previous full snapshot.
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
                .fetch_products_page_with_user_agent(shop_url, limit, cursor.as_deref(), None)
                .await?;

            all_products.extend(response.products);

            cursor = extract_next_cursor(link_header.as_deref());
            if cursor.is_none() {
                break;
            }
        }

        Ok(all_products)
    }

    /// Fetches all products using a browser-like request profile to bypass
    /// storefront bot filtering that blocks default scraper headers.
    ///
    /// This path is intended as a fallback for known strict stores that return
    /// 403 to normal requests.
    ///
    /// # Errors
    ///
    /// Returns [`ScraperError`] if the HTTP request fails or the response cannot be parsed.
    pub async fn fetch_all_products_browser_profile(
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
                .fetch_products_page_with_user_agent(
                    shop_url,
                    limit,
                    cursor.as_deref(),
                    Some(super::BROWSER_FALLBACK_UA),
                )
                .await?;

            all_products.extend(response.products);

            cursor = extract_next_cursor(link_header.as_deref());
            if cursor.is_none() {
                break;
            }
        }

        Ok(all_products)
    }
}
