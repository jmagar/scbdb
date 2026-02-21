//! Domain types for store locator extraction.

/// A store location record extracted from a brand's store locator page.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawStoreLocation {
    /// Provider-assigned store ID, if available.
    pub external_id: Option<String>,
    /// Store display name.
    pub name: String,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub phone: Option<String>,
    /// Which extraction strategy produced this record.
    ///
    /// One of: `"locally"`, `"storemapper"`, `"jsonld"`, `"json_embed"`.
    pub locator_source: String,
    /// The raw provider JSON object for this store, preserved for debugging.
    pub raw_data: serde_json::Value,
}

/// Errors that can occur while fetching or parsing a store locator page.
#[derive(Debug, thiserror::Error)]
pub enum LocatorError {
    #[error("HTTP error fetching locator page: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}
