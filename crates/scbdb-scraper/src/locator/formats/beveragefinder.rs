//! BeverageFinder.net embed extraction.

use regex::Regex;

use crate::locator::types::{LocatorError, RawStoreLocation};

/// Extract `BeverageFinder` key from embed script HTML.
pub(in crate::locator) fn extract_beveragefinder_key(html: &str) -> Option<String> {
    if !html.contains("beveragefinder") {
        return None;
    }

    let key_re =
        Regex::new(r#"beveragefinder\.net/users/embed\.js[^>]*data-key\s*=\s*[\"']([^\"']+)[\"']"#)
            .expect("valid regex");
    if let Some(cap) = key_re.captures(html) {
        if let Some(m) = cap.get(1) {
            return Some(m.as_str().to_string());
        }
    }

    let iframe_re =
        Regex::new(r#"beveragefinder-map\.php\?[^\"'\s>]*key=([^&\"'\s>]+)"#).expect("valid regex");
    iframe_re
        .captures(&html.replace("&amp;", "&"))
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

/// Fetch `BeverageFinder` results and parse the embedded `data-locations` JSON.
pub(in crate::locator) async fn fetch_beveragefinder_stores(
    client: &reqwest::Client,
    key: &str,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let config_response = client
        .get("https://beveragefinder.net/users/beveragefinder-map.php")
        .header(reqwest::header::USER_AGENT, user_agent)
        .query(&[("key", key), ("embed", "1")])
        .send()
        .await?
        .text()
        .await?;
    let config: serde_json::Value = match serde_json::from_str(&config_response) {
        Ok(value) => value,
        Err(error) => {
            tracing::debug!(key, %error, "beveragefinder map config was not valid JSON");
            serde_json::Value::Null
        }
    };

    let default_zip = config
        .get("defaultZip")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("10001");

    let form = [
        ("zip", default_zip),
        ("miles", "100"),
        ("brand", ""),
        ("key", key),
    ];

    let search_payload = client
        .post("https://beveragefinder.net/users/embed-search.php")
        .header(reqwest::header::USER_AGENT, user_agent)
        .form(&form)
        .send()
        .await?
        .text()
        .await?;

    let Some(html) = extract_search_html(&search_payload) else {
        tracing::debug!(key, "beveragefinder search payload was empty/unparseable");
        return Ok(vec![]);
    };

    if html.trim().is_empty() {
        tracing::debug!(key, "beveragefinder search response had empty html payload");
        return Ok(vec![]);
    }

    let Some(locations_json) = extract_data_locations_json(&html) else {
        tracing::debug!(key, "beveragefinder html missing data-locations attribute");
        return Ok(vec![]);
    };

    let values: serde_json::Value = match serde_json::from_str(&locations_json) {
        Ok(value) => value,
        Err(error) => {
            tracing::debug!(key, %error, "beveragefinder data-locations payload was invalid JSON");
            return Ok(vec![]);
        }
    };
    let Some(arr) = values.as_array() else {
        return Ok(vec![]);
    };

    Ok(arr.iter().filter_map(map_store_entry).collect())
}

fn map_store_entry(store: &serde_json::Value) -> Option<RawStoreLocation> {
    let name = store
        .get("name")
        .or_else(|| store.get("store"))
        .and_then(serde_json::Value::as_str)?
        .trim()
        .to_string();
    if name.is_empty() {
        return None;
    }

    Some(RawStoreLocation {
        external_id: store.get("id").and_then(|v| {
            if v.is_null() {
                return None;
            }
            v.as_str()
                .map(str::to_string)
                .or_else(|| Some(v.to_string()))
        }),
        name,
        address_line1: store
            .get("address")
            .or_else(|| store.get("address1"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        city: store
            .get("city")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        state: store
            .get("state")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        zip: store
            .get("zip")
            .or_else(|| store.get("postal"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        country: store
            .get("country")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
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
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        locator_source: "beveragefinder".to_string(),
        raw_data: store.clone(),
    })
}

fn extract_data_locations_json(html: &str) -> Option<String> {
    let re = Regex::new(r"data-locations='([^']*)'").expect("valid regex");
    let encoded = re
        .captures(html)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))?;
    Some(
        encoded
            .replace("&quot;", "\"")
            .replace("&#34;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&"),
    )
}

fn extract_search_html(search_payload: &str) -> Option<String> {
    let trimmed = search_payload.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(html) = payload.get("html").and_then(serde_json::Value::as_str) {
            return Some(html.to_string());
        }
    }

    if trimmed.contains("data-locations=") {
        return Some(trimmed.to_string());
    }

    None
}

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
}

#[cfg(test)]
mod tests {
    use super::{extract_beveragefinder_key, extract_data_locations_json, extract_search_html};

    #[test]
    fn extracts_key_from_embed_script() {
        let html = r#"<script src="https://beveragefinder.net/users/embed.js" data-key="abc123"></script>"#;
        assert_eq!(extract_beveragefinder_key(html).as_deref(), Some("abc123"));
    }

    #[test]
    fn extracts_locations_array_from_html_attribute() {
        let html = "<div id='map' data-locations='[{&quot;name&quot;:&quot;A&quot;}]'></div>";
        assert_eq!(
            extract_data_locations_json(html).as_deref(),
            Some(r#"[{"name":"A"}]"#)
        );
    }

    #[test]
    fn extract_search_html_handles_empty_payload() {
        assert_eq!(extract_search_html("   "), None);
    }

    #[test]
    fn extract_search_html_handles_json_payload() {
        let payload = r#"{"html":"<div id='map' data-locations='[]'></div>"}"#;
        assert_eq!(
            extract_search_html(payload).as_deref(),
            Some("<div id='map' data-locations='[]'></div>")
        );
    }

    #[test]
    fn extract_search_html_falls_back_to_raw_html() {
        let payload = "<div id='map' data-locations='[]'></div>";
        assert_eq!(extract_search_html(payload).as_deref(), Some(payload));
    }
}
