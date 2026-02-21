//! `StoreRocket` locator extraction.

use regex::Regex;

use crate::locator::types::{LocatorError, RawStoreLocation};

const MAX_SCRIPT_PROBES: usize = 4;

/// Extract a `StoreRocket` account/project ID from page HTML.
///
/// Recognized patterns:
/// - `StoreRocket.init({ ..., account: "Or85AG58NM", ... })`
/// - `data-storerocket-account="Or85AG58NM"`
/// - `storerocket.io/api/user/Or85AG58NM`
pub(in crate::locator) fn extract_storerocket_account(html: &str) -> Option<String> {
    if !html.to_ascii_lowercase().contains("storerocket") {
        return None;
    }

    let patterns = [
        r#"account\s*:\s*["']([A-Za-z0-9_-]{4,64})["']"#,
        r#"data-storerocket-account\s*=\s*["']([A-Za-z0-9_-]{4,64})["']"#,
        r"storerocket(?:\.io|\.test)/api/user/([A-Za-z0-9_-]{4,64})",
    ];

    for pattern in patterns {
        let re = Regex::new(pattern).expect("valid regex");
        if let Some(captures) = re.captures(html) {
            if let Some(value) = captures.get(1).map(|m| m.as_str().trim()) {
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

/// Discover a `StoreRocket` account ID from page HTML and linked JS bundles.
///
/// This is needed for modern SPA locators that render `StoreRocket.init(...)`
/// inside a compiled route chunk instead of the main HTML document.
pub(in crate::locator) async fn discover_storerocket_account(
    html: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Option<String> {
    if let Some(account) = extract_storerocket_account(html) {
        return Some(account);
    }

    let script_urls = extract_script_urls_for_account_probe(html);
    for script_url in script_urls.iter().take(MAX_SCRIPT_PROBES) {
        match crate::locator::fetch::fetch_text(script_url, timeout_secs, user_agent).await {
            Ok(script_body) => {
                if let Some(account) = extract_storerocket_account(&script_body) {
                    return Some(account);
                }
            }
            Err(error) => {
                tracing::debug!(
                    script_url,
                    %error,
                    "failed fetching candidate StoreRocket script"
                );
            }
        }
    }

    None
}

/// Fetch stores from the `StoreRocket` locations endpoint.
pub(in crate::locator) async fn fetch_storerocket_stores(
    account: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let url = format!("https://storerocket.io/api/user/{account}/locations");
    let payload = crate::locator::fetch::fetch_json(&url, timeout_secs, user_agent).await?;
    Ok(parse_storerocket_locations(&payload))
}

fn parse_storerocket_locations(payload: &serde_json::Value) -> Vec<RawStoreLocation> {
    extract_locations_array(payload)
        .into_iter()
        .flat_map(|locations| locations.iter())
        .filter_map(map_location)
        .collect()
}

fn extract_locations_array(payload: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    payload
        .get("results")
        .and_then(|results| results.get("locations"))
        .and_then(serde_json::Value::as_array)
        .or_else(|| {
            payload
                .get("locations")
                .and_then(serde_json::Value::as_array)
        })
}

fn map_location(store: &serde_json::Value) -> Option<RawStoreLocation> {
    let name = store
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)?;

    Some(RawStoreLocation {
        external_id: store
            .get("obf_id")
            .or_else(|| store.get("id"))
            .and_then(value_as_string),
        name,
        address_line1: store
            .get("address")
            .or_else(|| store.get("display_address"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        city: store
            .get("city")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        state: store
            .get("state")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        zip: store
            .get("zip")
            .or_else(|| store.get("postal"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        country: store
            .get("country")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        latitude: store
            .get("lat")
            .or_else(|| store.get("latitude"))
            .and_then(value_as_f64),
        longitude: store
            .get("lng")
            .or_else(|| store.get("longitude"))
            .and_then(value_as_f64),
        phone: store
            .get("phone")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        locator_source: "storerocket".to_string(),
        raw_data: store.clone(),
    })
}

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
}

fn value_as_string(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(str::to_string).or_else(|| {
        if value.is_number() {
            Some(value.to_string())
        } else {
            None
        }
    })
}

fn extract_script_urls_for_account_probe(html: &str) -> Vec<String> {
    let script_url_re =
        Regex::new(r#"https?://[^\s"'<>]+\.js(?:\?[^\s"'<>]*)?"#).expect("valid regex");
    let mut urls = std::collections::BTreeSet::new();

    for capture in script_url_re.find_iter(html) {
        let url = capture.as_str();
        let lowered = url.to_ascii_lowercase();
        if lowered.contains("storerocket")
            || lowered.contains("store-locator")
            || lowered.contains("locator")
        {
            urls.insert(url.to_string());
        }
    }

    urls.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        extract_script_urls_for_account_probe, extract_storerocket_account,
        parse_storerocket_locations,
    };

    #[test]
    fn extracts_account_from_init_call() {
        let html = r#"
            <script>
                window.StoreRocket.init({selector: ".storerocket-store-locator", account: "Or85AG58NM"});
            </script>
        "#;

        assert_eq!(
            extract_storerocket_account(html).as_deref(),
            Some("Or85AG58NM")
        );
    }

    #[test]
    fn extracts_account_from_data_attribute() {
        let html = r#"<div id="storerocket-widget" data-storerocket-account="abc123XYZ"></div>"#;

        assert_eq!(
            extract_storerocket_account(html).as_deref(),
            Some("abc123XYZ")
        );
    }

    #[test]
    fn parses_results_locations_payload() {
        let payload = json!({
            "success": true,
            "results": {
                "locations": [
                    {
                        "obf_id": "2876ZQdNJA",
                        "name": "Sample Store",
                        "address": "123 Main St, Austin, TX 78701, US",
                        "city": "Austin",
                        "state": "TX",
                        "zip": "78701",
                        "country": "US",
                        "lat": "30.2672",
                        "lng": "-97.7431",
                        "phone": "512-555-0100"
                    }
                ]
            }
        });

        let locations = parse_storerocket_locations(&payload);
        assert_eq!(locations.len(), 1);
        let location = &locations[0];

        assert_eq!(location.external_id.as_deref(), Some("2876ZQdNJA"));
        assert_eq!(location.name, "Sample Store");
        assert_eq!(location.city.as_deref(), Some("Austin"));
        assert_eq!(location.state.as_deref(), Some("TX"));
        assert_eq!(location.zip.as_deref(), Some("78701"));
        assert_eq!(location.country.as_deref(), Some("US"));
        assert_eq!(location.phone.as_deref(), Some("512-555-0100"));
        assert_eq!(location.locator_source, "storerocket");
        assert_eq!(location.latitude, Some(30.2672));
        assert_eq!(location.longitude, Some(-97.7431));
    }

    #[test]
    fn skips_entries_without_name() {
        let payload = json!({
            "results": {
                "locations": [
                    {"name": "", "obf_id": "missing-name"},
                    {"obf_id": "missing-name-2"},
                    {"name": "Valid", "obf_id": "valid"}
                ]
            }
        });

        let locations = parse_storerocket_locations(&payload);
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].name, "Valid");
        assert_eq!(locations[0].external_id.as_deref(), Some("valid"));
    }

    #[test]
    fn extracts_candidate_script_urls_from_html() {
        let html = r#"
            <link rel="modulepreload" href="https://cdn.shopify.com/assets/store-locator-abc123.js" />
            <script src="https://cdn.storerocket.io/widget.js"></script>
            <script src="https://cdn.shopify.com/assets/other.js"></script>
        "#;

        let urls = extract_script_urls_for_account_probe(html);
        assert_eq!(urls.len(), 2);
        assert!(urls
            .iter()
            .any(|url| url == "https://cdn.shopify.com/assets/store-locator-abc123.js"));
        assert!(urls
            .iter()
            .any(|url| url == "https://cdn.storerocket.io/widget.js"));
    }
}
