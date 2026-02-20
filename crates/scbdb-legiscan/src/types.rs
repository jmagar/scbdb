//! `LegiScan` API response types.
//!
//! All types model the JSON structures returned by the `LegiScan` REST API.
//! The API wraps every response in a `{"status": "OK", ...}` envelope;
//! [`ApiResponse`] captures that pattern generically.

use std::collections::HashMap;

use serde::Deserialize;

/// Top-level envelope for all `LegiScan` API responses.
///
/// The `status` field is `"OK"` on success or `"ERROR"` on failure.
/// The remaining fields are flattened from the response body.
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub status: String,
    #[serde(flatten)]
    pub data: T,
}

// ---------------------------------------------------------------------------
// getBill
// ---------------------------------------------------------------------------

/// Wrapper for the `getBill` response: `{ "bill": { ... } }`.
#[derive(Debug, Deserialize)]
pub struct BillResponse {
    pub bill: BillDetail,
}

/// Full detail for a single bill returned by `getBill`.
#[derive(Debug, Deserialize)]
pub struct BillDetail {
    pub bill_id: i64,
    pub bill_number: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Numeric status code: 1=introduced, 2=engrossed, 3=enrolled,
    /// 4=passed, 5=vetoed, 6=failed.
    pub status: i32,
    #[serde(default)]
    pub status_date: Option<String>,
    pub state: String,
    #[serde(default)]
    pub session: Option<SessionDetail>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub history: Vec<BillHistory>,
    #[serde(default)]
    pub progress: Vec<BillProgress>,
}

/// Session metadata embedded inside a [`BillDetail`].
#[derive(Debug, Deserialize)]
pub struct SessionDetail {
    pub session_id: i64,
    pub session_name: String,
    pub year_start: i32,
    pub year_end: i32,
}

/// A single legislative action in a bill's history.
#[derive(Debug, Deserialize)]
pub struct BillHistory {
    /// Date of the action in `"YYYY-MM-DD"` format.
    pub date: String,
    pub action: String,
    #[serde(default)]
    pub chamber: Option<String>,
}

/// A progress milestone for a bill.
#[derive(Debug, Deserialize)]
pub struct BillProgress {
    pub date: String,
    pub event: i32,
}

// ---------------------------------------------------------------------------
// search
// ---------------------------------------------------------------------------

/// Wrapper for the `search` response.
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub searchresult: SearchResult,
}

/// The inner search result containing summary metadata and matching bills.
///
/// The `LegiScan` `search` endpoint returns bills as numbered string keys
/// (`"0"`, `"1"`, …) alongside the `summary` key — matching the `getMasterList`
/// envelope shape. `results` captures all remaining keys via `#[serde(flatten)]`;
/// callers must filter numeric keys and deserialize each value individually.
#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub summary: SearchSummary,
    #[serde(flatten)]
    pub results: HashMap<String, serde_json::Value>,
}

/// Pagination and relevance metadata from a search response.
///
/// The `LegiScan` API returns `page` as a human-readable string (`"1 of 451"`),
/// `range` as an ordinal range string (`"1 - 50"`), and `relevancy` as a
/// percentage range string (`"100% - 99%"`). All three are `String` to match
/// the actual wire format — treating them as `i32` causes deserialization failure.
#[derive(Debug, Deserialize)]
pub struct SearchSummary {
    #[serde(default)]
    pub page: String,
    #[serde(default)]
    pub range: String,
    #[serde(default)]
    pub relevancy: String,
    pub count: i32,
    #[serde(default)]
    pub page_current: Option<i32>,
    #[serde(default)]
    pub page_total: Option<i32>,
}

/// A single bill returned by a search query.
#[derive(Debug, Deserialize)]
pub struct BillSearchItem {
    pub bill_id: i64,
    pub bill_number: String,
    pub title: String,
    pub state: String,
    pub status: i32,
    #[serde(default)]
    pub status_date: Option<String>,
    #[serde(default)]
    pub last_action_date: Option<String>,
    #[serde(default)]
    pub last_action: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

// ---------------------------------------------------------------------------
// getSessionList
// ---------------------------------------------------------------------------

/// Wrapper for the `getSessionList` response.
#[derive(Debug, Deserialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionInfo>,
}

/// A legislative session returned by `getSessionList`.
#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    pub session_id: i64,
    pub state_id: i32,
    pub year_start: i32,
    pub year_end: i32,
    pub session_name: String,
    #[serde(default)]
    pub special: i32,
    #[serde(default)]
    pub prior: i32,
    #[serde(default)]
    pub sine_die: i32,
}

// ---------------------------------------------------------------------------
// getMasterList
// ---------------------------------------------------------------------------

/// Wrapper for the `getMasterList` response.
#[derive(Debug, Deserialize)]
pub struct MasterListResponse {
    pub masterlist: MasterListData,
}

/// The master list payload, containing session info and bills keyed by
/// numeric strings (`"0"`, `"1"`, etc.). The `"session"` key is a
/// [`SessionDetail`]; all other keys are bill entries.
#[derive(Debug, Deserialize)]
pub struct MasterListData {
    pub session: SessionDetail,
    #[serde(flatten)]
    pub bills: HashMap<String, serde_json::Value>,
}

/// A single entry from the master bill list.
#[derive(Debug, Deserialize)]
pub struct MasterListEntry {
    pub bill_id: i64,
    pub number: String,
    pub title: String,
    pub status: i32,
    #[serde(default)]
    pub status_date: Option<String>,
    #[serde(default)]
    pub last_action_date: Option<String>,
    #[serde(default)]
    pub last_action: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    pub change_hash: String,
}
