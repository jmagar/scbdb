//! Strategy 1: Locally.com widget extraction.

use regex::Regex;

use crate::locator::fetch::fetch_json;
use crate::locator::types::{LocatorError, RawStoreLocation};

/// Extract the Locally.com company ID from HTML.
///
/// Recognises patterns such as:
/// - `company_id=12345`
/// - `locallyWidgetCompanyId = 12345`
/// - `locally.com/stores/json?...company_id=12345`
pub(in crate::locator) fn extract_locally_company_id(html: &str) -> Option<String> {
    // Use specific Locally signals only; bare `company_id` is too generic and
    // would false-positive on CRM/analytics pages.
    if !html.contains("locally.com") && !html.contains("locallyWidgetCompanyId") {
        return None;
    }

    // Ordered from most specific to most general.
    let patterns = [
        r#"locally\.com/stores/json\?[^"']*company_id=(\d+)"#,
        r"locallyWidgetCompanyId\s*[=:]\s*(\d+)",
        r"company_id\s*[=:]\s*(\d+)",
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

/// Fetch stores from the Locally.com JSON API and map them to `RawStoreLocation`.
pub(in crate::locator) async fn fetch_locally_stores(
    company_id: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let url = format!("https://api.locally.com/stores/json?company_id={company_id}&take=10000");
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
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                country: store
                    .get("country")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                latitude: store
                    .get("lat")
                    .or_else(|| store.get("latitude"))
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                    }),
                longitude: store
                    .get("lng")
                    .or_else(|| store.get("longitude"))
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                    }),
                phone: store
                    .get("phone")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                locator_source: "locally".to_string(),
                raw_data: store,
            })
        })
        .collect();

    Ok(locations)
}
