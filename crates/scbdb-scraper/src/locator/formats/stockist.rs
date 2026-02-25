//! Strategy 3: Stockist widget extraction.

use regex::Regex;

use crate::locator::types::{LocatorError, RawStoreLocation};

/// Extract the Stockist widget tag from HTML.
///
/// Recognises patterns such as:
/// - `data-stockist-widget-tag="u23010"`
/// - `stockist.co/api/v1/u23010/...`
/// - `_stockistConfigCallback_u23010(...)`
pub(in crate::locator) fn extract_stockist_widget_tag(html: &str) -> Option<String> {
    if !html.contains("stockist") {
        return None;
    }

    let patterns = [
        r#"data-stockist-widget-tag\s*=\s*["']([^"']+)["']"#,
        r"stockist\.co/api/v1/([A-Za-z0-9_-]+)/",
        r"_stockistConfigCallback_([A-Za-z0-9_-]+)",
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

/// Fetch stores from the Stockist API and map them to `RawStoreLocation`.
pub(in crate::locator) async fn fetch_stockist_stores(
    client: &reqwest::Client,
    tag: &str,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let config = fetch_stockist_config(client, tag, user_agent).await?;

    let latitude = json_number_or_string(&config, "latitude").unwrap_or(39.828_175);
    let longitude = json_number_or_string(&config, "longitude").unwrap_or(-98.579_5);
    let distance = config
        .get("max_distance")
        .or_else(|| config.get("distance"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(50_000);

    let search_url = format!(
        "https://stockist.co/api/v1/{tag}/locations/search?latitude={latitude}&longitude={longitude}&distance={distance}&units=mi&page=1&per_page=10000"
    );

    let data = crate::locator::fetch::fetch_json(client, &search_url, user_agent).await?;

    let Some(stores) = data.get("locations").and_then(serde_json::Value::as_array) else {
        return Ok(vec![]);
    };

    let locations = stores
        .iter()
        .filter_map(|store| {
            let name = store.get("name")?.as_str()?.trim().to_string();
            if name.is_empty() {
                return None;
            }

            Some(RawStoreLocation {
                external_id: store.get("id").and_then(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .or_else(|| Some(v.to_string()))
                }),
                name,
                address_line1: store
                    .get("address_line_1")
                    .or_else(|| store.get("full_address"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                city: store
                    .get("city")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                state: store
                    .get("state")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                zip: store
                    .get("postal_code")
                    .or_else(|| store.get("zip"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                country: store
                    .get("country")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                latitude: store
                    .get("latitude")
                    .or_else(|| store.get("lat"))
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                    }),
                longitude: store
                    .get("longitude")
                    .or_else(|| store.get("lng"))
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                    }),
                phone: store
                    .get("phone")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                locator_source: "stockist".to_string(),
                raw_data: store.clone(),
            })
        })
        .collect();

    Ok(locations)
}

async fn fetch_stockist_config(
    client: &reqwest::Client,
    tag: &str,
    user_agent: &str,
) -> Result<serde_json::Value, LocatorError> {
    let url = format!(
        "https://stockist.co/api/v1/{tag}/widget.js?callback=_stockistConfigCallback_{tag}"
    );
    let body = crate::locator::fetch::fetch_text(client, &url, user_agent).await?;

    let open = body.find('(').unwrap_or(0);
    let close = body.rfind(')').unwrap_or(body.len());

    if open >= close || close > body.len() {
        return Ok(serde_json::Value::Null);
    }

    let payload = body[open + 1..close].trim();
    let value: serde_json::Value = serde_json::from_str(payload)?;
    Ok(value)
}

fn json_number_or_string(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

#[cfg(test)]
mod tests {
    use super::extract_stockist_widget_tag;

    #[test]
    fn extracts_tag_from_data_attribute() {
        let html = r#"<div data-stockist-widget-tag="u23010"></div>"#;
        assert_eq!(extract_stockist_widget_tag(html).as_deref(), Some("u23010"));
    }

    #[test]
    fn extracts_tag_from_api_url() {
        let html = r#"<script src="https://stockist.co/api/v1/u12345/widget.js"></script>"#;
        assert_eq!(extract_stockist_widget_tag(html).as_deref(), Some("u12345"));
    }
}
