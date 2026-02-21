//! HTTP client for the `LegiScan` REST API.
//!
//! Wraps `reqwest` with error handling, API key management, typed response
//! deserialization, per-session request budget enforcement, and exponential
//! back-off retry for transient errors. All endpoints surface quota exhaustion
//! as [`LegiscanError::QuotaExceeded`] so callers can stop immediately rather
//! than burning through the monthly API budget on doomed retries.
//!
//! Session and master-list endpoints live in [`super::session`].

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use reqwest::{Client, Url};

use crate::error::LegiscanError;
use crate::retry::retry_with_backoff;
use crate::types::{ApiResponse, BillDetail, BillResponse, BillSearchItem, SearchResponse};

const DEFAULT_BASE_URL: &str = "https://api.legiscan.com/";

/// Client for the `LegiScan` REST API.
///
/// Tracks a per-session request counter and enforces `max_requests` to protect
/// the monthly API quota. Once the ceiling is reached every method returns
/// [`LegiscanError::BudgetExceeded`] immediately — no network traffic.
pub struct LegiscanClient {
    client: Client,
    api_key: String,
    base_url: Url,
    /// Monotonically increasing count of HTTP requests issued this session.
    request_count: AtomicU32,
    /// Hard ceiling on HTTP requests for this session.
    max_requests: u32,
}

impl LegiscanClient {
    /// Creates a new client pointed at the production `LegiScan` API.
    ///
    /// `max_requests` is enforced as a hard ceiling: once reached all further
    /// calls return [`LegiscanError::BudgetExceeded`] without making any
    /// network request. Set to a value consistent with your monthly plan
    /// (30 000 requests / expected runs per month).
    ///
    /// # Errors
    ///
    /// Returns [`LegiscanError::Http`] if the `reqwest::Client` cannot be built.
    pub fn new(api_key: &str, timeout_secs: u64, max_requests: u32) -> Result<Self, LegiscanError> {
        Self::with_base_url(api_key, timeout_secs, max_requests, DEFAULT_BASE_URL)
    }

    /// Creates a new client with a custom base URL (for testing with wiremock).
    ///
    /// # Errors
    ///
    /// Returns [`LegiscanError::Http`] if the client cannot be built, or
    /// [`LegiscanError::ApiError`] if `base_url` is not a valid URL.
    pub fn with_base_url(
        api_key: &str,
        timeout_secs: u64,
        max_requests: u32,
        base_url: &str,
    ) -> Result<Self, LegiscanError> {
        let connect_timeout = timeout_secs.min(10);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(connect_timeout))
            .user_agent("scbdb/0.1 (regulatory-tracking)")
            .build()?;
        let normalised = format!("{}/", base_url.trim_end_matches('/'));
        let base_url = Url::parse(&normalised)
            .map_err(|e| LegiscanError::ApiError(format!("invalid base URL '{base_url}': {e}")))?;
        Ok(Self {
            client,
            api_key: api_key.to_owned(),
            base_url,
            request_count: AtomicU32::new(0),
            max_requests,
        })
    }

    /// Returns the number of HTTP requests issued by this client so far.
    #[must_use]
    pub fn requests_used(&self) -> u32 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Fetches full bill details by `LegiScan` bill ID.
    ///
    /// # Errors
    ///
    /// - [`LegiscanError::BudgetExceeded`] if the session request budget is reached.
    /// - [`LegiscanError::QuotaExceeded`] if `LegiScan`'s API quota is exhausted.
    /// - [`LegiscanError::ApiError`] on API-level failure.
    /// - [`LegiscanError::Http`] on network or non-2xx HTTP failure.
    /// - [`LegiscanError::Deserialize`] if the response shape is unexpected.
    pub async fn get_bill(&self, bill_id: i64) -> Result<BillDetail, LegiscanError> {
        let url = self.build_url("getBill", &[("id", &bill_id.to_string())]);
        let body = self.request_json(&url).await?;
        Self::check_api_error(&body)?;
        let envelope: ApiResponse<BillResponse> =
            serde_json::from_value(body).map_err(|e| LegiscanError::Deserialize {
                context: format!("getBill(id={bill_id})"),
                source: e,
            })?;
        Ok(envelope.data.bill)
    }

    /// Searches for bills by keyword and optional state, fetching up to
    /// `max_pages` pages of results (50 bills per page).
    ///
    /// Pass `state = Some("US")` for US Congress bills.
    /// Pass `state = None` to search all states.
    /// Pagination stops early if the API reports no more pages or a page
    /// returns no results.
    ///
    /// Each page consumes one request from the session budget.
    ///
    /// # Errors
    ///
    /// - [`LegiscanError::BudgetExceeded`] if the session budget is reached.
    /// - [`LegiscanError::QuotaExceeded`] if `LegiScan`'s quota is exhausted.
    /// - [`LegiscanError::ApiError`] on API-level failure.
    /// - [`LegiscanError::Http`] on network failure.
    /// - [`LegiscanError::Deserialize`] if the response shape is unexpected.
    pub async fn search_bills(
        &self,
        query: &str,
        state: Option<&str>,
        max_pages: u32,
    ) -> Result<Vec<BillSearchItem>, LegiscanError> {
        let mut all_items: Vec<BillSearchItem> = Vec::new();
        let state_upper = state.map(str::to_uppercase);

        for page in 1..=max_pages.max(1) {
            let page_str = page.to_string();
            let mut params = vec![("query", query), ("page", page_str.as_str())];
            if let Some(ref s) = state_upper {
                params.push(("state", s.as_str()));
            }

            let url = self.build_url("search", &params);
            let body = self.request_json(&url).await?;
            Self::check_api_error(&body)?;

            let envelope: ApiResponse<SearchResponse> =
                serde_json::from_value(body).map_err(|e| LegiscanError::Deserialize {
                    context: format!("search(query={query}, page={page})"),
                    source: e,
                })?;

            let summary = &envelope.data.searchresult.summary;
            let page_total = u32::try_from(summary.page_total.unwrap_or(1).max(1)).unwrap_or(1);

            let page_items: Vec<BillSearchItem> = envelope
                .data
                .searchresult
                .results
                .into_iter()
                .filter(|(k, _)| k.parse::<u32>().is_ok())
                .filter_map(|(k, v)| {
                    serde_json::from_value::<BillSearchItem>(v)
                        .map_err(|e| {
                            tracing::warn!(key = %k, error = %e, "search_bills: skipping malformed entry");
                        })
                        .ok()
                })
                .collect();

            let done = page_items.is_empty() || page >= page_total;
            all_items.extend(page_items);
            if done {
                break;
            }
        }

        Ok(all_items)
    }

    pub(crate) fn build_url(&self, op: &str, extra: &[(&str, &str)]) -> Url {
        let mut url = self.base_url.clone();
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("key", &self.api_key);
            pairs.append_pair("op", op);
            for (k, v) in extra {
                pairs.append_pair(k, v);
            }
        }
        url
    }

    /// Issues a GET request with exponential-backoff retries on transient errors.
    ///
    /// The request budget is checked and incremented before the first attempt.
    /// If the ceiling is already reached, returns [`LegiscanError::BudgetExceeded`]
    /// without making any network request. Each logical call counts as one
    /// budget unit regardless of how many retry attempts succeed internally.
    pub(crate) async fn request_json(&self, url: &Url) -> Result<serde_json::Value, LegiscanError> {
        let used = self.request_count.fetch_add(1, Ordering::Relaxed);
        if used >= self.max_requests {
            self.request_count.fetch_sub(1, Ordering::Relaxed);
            return Err(LegiscanError::BudgetExceeded {
                used,
                limit: self.max_requests,
            });
        }
        tracing::debug!(
            requests_used = used + 1,
            max = self.max_requests,
            url = %url,
            "LegiScan request"
        );
        let client = self.client.clone();
        let url = url.clone();
        retry_with_backoff(3, 1_000, move || {
            let client = client.clone();
            let url = url.clone();
            async move {
                let response = client.get(url.clone()).send().await?;
                let response = response.error_for_status()?;
                let body = response.text().await?;
                serde_json::from_str(&body).map_err(|e| LegiscanError::Deserialize {
                    context: url.to_string(),
                    source: e,
                })
            }
        })
        .await
    }

    /// Checks the top-level `"status"` field and returns an error on failure.
    ///
    /// Quota-exhaustion messages (`"limit exceeded"`, `"access denied"`) are
    /// surfaced as [`LegiscanError::QuotaExceeded`] — a non-retriable hard stop.
    /// All other error messages become [`LegiscanError::ApiError`].
    pub(crate) fn check_api_error(body: &serde_json::Value) -> Result<(), LegiscanError> {
        if body.get("status").and_then(serde_json::Value::as_str) == Some("ERROR") {
            let msg = body
                .get("alert")
                .and_then(|a| a.get("message"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown error");
            let lower = msg.to_ascii_lowercase();
            if lower.contains("limit exceeded") || lower.contains("access denied") {
                return Err(LegiscanError::QuotaExceeded(msg.to_string()));
            }
            return Err(LegiscanError::ApiError(msg.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "client_test.rs"]
mod tests;
