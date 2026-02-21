//! Strategy 2: Storemapper widget extraction.

use regex::Regex;

use crate::locator::fetch::fetch_json;
use crate::locator::types::{LocatorError, RawStoreLocation};

/// Extract the Storemapper API token from HTML.
///
/// Recognises patterns such as:
/// - `data-storemapper-token="abc123"`
/// - `storemapper.co/api/stores?token=abc123`
/// - `token: "abc123"` near a storemapper reference
pub(in crate::locator) fn extract_storemapper_token(html: &str) -> Option<String> {
    if !html.contains("storemapper") {
        return None;
    }

    let patterns = [
        r#"storemapper\.co/api/stores\?token=([^"'&\s]+)"#,
        r#"data-storemapper-token\s*=\s*["']([^"']+)["']"#,
        r#"token["'\s:=]+([A-Za-z0-9_-]{8,})"#,
    ];

    for pattern in &patterns {
        let re = Regex::new(pattern).expect("valid regex");
        if let Some(cap) = re.captures(html) {
            if let Some(m) = cap.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }
    None
}

/// Fetch stores from the Storemapper API and map them to `RawStoreLocation`.
pub(in crate::locator) async fn fetch_storemapper_stores(
    token: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let url = format!("https://storemapper.co/api/stores?token={token}");
    let data = fetch_json(&url, timeout_secs, user_agent).await?;

    let stores = match data.get("stores").and_then(|v| v.as_array()) {
        Some(arr) => arr.clone(),
        None => return Ok(vec![]),
    };

    let locations = stores
        .into_iter()
        .filter_map(|store| {
            let name = store.get("name")?.as_str()?.to_string();
            Some(RawStoreLocation {
                external_id: store.get("id").and_then(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .or_else(|| Some(v.to_string()))
                }),
                name,
                address_line1: store
                    .get("address")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                city: store
                    .get("city")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                state: store
                    .get("state")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                zip: store
                    .get("zip")
                    .or_else(|| store.get("postal_code"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                country: store
                    .get("country")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                latitude: store
                    .get("lat")
                    .or_else(|| store.get("latitude"))
                    .and_then(serde_json::Value::as_f64),
                longitude: store
                    .get("lng")
                    .or_else(|| store.get("longitude"))
                    .and_then(serde_json::Value::as_f64),
                phone: store
                    .get("phone")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                locator_source: "storemapper".to_string(),
                raw_data: store,
            })
        })
        .collect();

    Ok(locations)
}
