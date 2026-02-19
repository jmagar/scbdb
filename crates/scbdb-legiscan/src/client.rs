//! HTTP client for the `LegiScan` REST API.
//!
//! Wraps `reqwest` with `LegiScan`-specific error handling, API key management,
//! and typed response deserialization. All endpoints check the `"status"` field
//! in the JSON envelope and surface API-level errors as [`LegiscanError::ApiError`].

use std::time::Duration;

use reqwest::Client;

use crate::error::LegiscanError;
use crate::types::{
    ApiResponse, BillDetail, BillResponse, BillSearchItem, MasterListEntry, MasterListResponse,
    SearchResponse, SessionDetail, SessionInfo, SessionListResponse,
};

const DEFAULT_BASE_URL: &str = "https://api.legiscan.com/";

/// Client for the `LegiScan` REST API.
///
/// Manages the HTTP client, API key, and base URL. Use [`LegiscanClient::new`]
/// for production or [`LegiscanClient::with_base_url`] to point at a mock
/// server in tests.
pub struct LegiscanClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl LegiscanClient {
    /// Creates a new client pointed at the production `LegiScan` API.
    ///
    /// # Errors
    ///
    /// Returns [`LegiscanError::Http`] if the underlying `reqwest::Client`
    /// cannot be constructed.
    pub fn new(api_key: &str, timeout_secs: u64) -> Result<Self, LegiscanError> {
        Self::with_base_url(api_key, timeout_secs, DEFAULT_BASE_URL)
    }

    /// Creates a new client with a custom base URL (for testing with wiremock).
    ///
    /// # Errors
    ///
    /// Returns [`LegiscanError::Http`] if the underlying `reqwest::Client`
    /// cannot be constructed.
    pub fn with_base_url(
        api_key: &str,
        timeout_secs: u64,
        base_url: &str,
    ) -> Result<Self, LegiscanError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("scbdb/0.1 (regulatory-tracking)")
            .build()?;
        Ok(Self {
            client,
            api_key: api_key.to_owned(),
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    /// Fetches full bill details by `LegiScan` bill ID.
    ///
    /// Calls the `getBill` endpoint and returns the parsed [`BillDetail`].
    ///
    /// # Errors
    ///
    /// - [`LegiscanError::ApiError`] if the API returns an error status.
    /// - [`LegiscanError::Http`] on network failure.
    /// - [`LegiscanError::Deserialize`] if the response does not match the
    ///   expected shape.
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

    /// Searches for bills by keyword and optional state filter.
    ///
    /// Calls the `search` endpoint (50 results/page, full bill metadata).
    /// When `state` is `None`, searches all states.
    ///
    /// # Errors
    ///
    /// - [`LegiscanError::ApiError`] if the API returns an error status.
    /// - [`LegiscanError::Http`] on network failure.
    /// - [`LegiscanError::Deserialize`] if the response does not match the
    ///   expected shape.
    pub async fn search_bills(
        &self,
        query: &str,
        state: Option<&str>,
    ) -> Result<Vec<BillSearchItem>, LegiscanError> {
        let mut params = vec![("query", query)];
        // Bind the owned string outside the if block so the borrow lives long enough.
        let state_upper;
        if let Some(s) = state {
            state_upper = s.to_uppercase();
            params.push(("state", &state_upper));
        }

        let url = self.build_url("search", &params);
        let body = self.request_json(&url).await?;
        Self::check_api_error(&body)?;

        let envelope: ApiResponse<SearchResponse> =
            serde_json::from_value(body).map_err(|e| LegiscanError::Deserialize {
                context: format!("searchRaw(query={query})"),
                source: e,
            })?;

        Ok(envelope.data.searchresult.results)
    }

    /// Gets the list of legislative sessions for a state (e.g., `"SC"`).
    ///
    /// # Errors
    ///
    /// - [`LegiscanError::ApiError`] if the API returns an error status.
    /// - [`LegiscanError::Http`] on network failure.
    /// - [`LegiscanError::Deserialize`] if the response does not match the
    ///   expected shape.
    pub async fn get_session_list(&self, state: &str) -> Result<Vec<SessionInfo>, LegiscanError> {
        let state_upper = state.to_uppercase();
        let url = self.build_url("getSessionList", &[("state", &state_upper)]);
        let body = self.request_json(&url).await?;
        Self::check_api_error(&body)?;

        let envelope: ApiResponse<SessionListResponse> =
            serde_json::from_value(body).map_err(|e| LegiscanError::Deserialize {
                context: format!("getSessionList(state={state})"),
                source: e,
            })?;

        Ok(envelope.data.sessions)
    }

    /// Gets the master bill list for a state's current session.
    ///
    /// Returns the session metadata and a list of bill entries. The `LegiScan`
    /// API returns bills as a `HashMap<String, Value>` with numeric string
    /// keys; this method parses each entry individually and skips any that
    /// fail to deserialize.
    ///
    /// # Errors
    ///
    /// - [`LegiscanError::ApiError`] if the API returns an error status.
    /// - [`LegiscanError::Http`] on network failure.
    /// - [`LegiscanError::Deserialize`] if the top-level response does not
    ///   match the expected shape.
    pub async fn get_master_list(
        &self,
        state: &str,
    ) -> Result<(SessionDetail, Vec<MasterListEntry>), LegiscanError> {
        let state_upper = state.to_uppercase();
        let url = self.build_url("getMasterList", &[("state", &state_upper)]);
        let body = self.request_json(&url).await?;
        Self::check_api_error(&body)?;

        let envelope: ApiResponse<MasterListResponse> =
            serde_json::from_value(body).map_err(|e| LegiscanError::Deserialize {
                context: format!("getMasterList(state={state})"),
                source: e,
            })?;

        let data = envelope.data.masterlist;
        let entries = data
            .bills
            .into_iter()
            .filter(|(k, _)| k != "session")
            .filter_map(|(_, v)| serde_json::from_value::<MasterListEntry>(v).ok())
            .collect();

        Ok((data.session, entries))
    }

    /// Builds the full request URL with query parameters.
    fn build_url(&self, op: &str, extra: &[(&str, &str)]) -> String {
        let mut url = format!("{}/?key={}&op={}", self.base_url, self.api_key, op);
        for (k, v) in extra {
            url.push('&');
            url.push_str(k);
            url.push('=');
            url.push_str(v);
        }
        url
    }

    /// Sends a GET request and parses the response body as JSON.
    async fn request_json(&self, url: &str) -> Result<serde_json::Value, LegiscanError> {
        let response = self.client.get(url).send().await?;
        let body = response.text().await?;
        serde_json::from_str(&body).map_err(|e| LegiscanError::Deserialize {
            context: url.to_string(),
            source: e,
        })
    }

    /// Checks the top-level `"status"` field and returns an error if it
    /// indicates failure.
    fn check_api_error(body: &serde_json::Value) -> Result<(), LegiscanError> {
        if body.get("status").and_then(serde_json::Value::as_str) == Some("ERROR") {
            let msg = body
                .get("alert")
                .and_then(|a| a.get("message"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown error")
                .to_string();
            return Err(LegiscanError::ApiError(msg));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client(base_url: &str) -> LegiscanClient {
        LegiscanClient::with_base_url("test-key", 30, base_url)
            .expect("client construction should not fail")
    }

    #[test]
    fn build_url_constructs_correct_query_string() {
        let client = test_client("https://api.legiscan.com");
        let url = client.build_url("getBill", &[("id", "42")]);
        assert_eq!(
            url,
            "https://api.legiscan.com/?key=test-key&op=getBill&id=42"
        );
    }

    #[test]
    fn build_url_strips_trailing_slash() {
        let client = test_client("https://api.legiscan.com/");
        let url = client.build_url("searchRaw", &[("query", "hemp"), ("state", "SC")]);
        assert_eq!(
            url,
            "https://api.legiscan.com/?key=test-key&op=searchRaw&query=hemp&state=SC"
        );
    }
}
