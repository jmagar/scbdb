//! Strategy 2: Storemapper widget extraction.

use std::sync::LazyLock;

use regex::Regex;

use crate::locator::types::{LocatorError, RawStoreLocation};

static TOKEN_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r#"storemapper\.co/api/stores\?token=([^"'&\s]+)"#).expect("valid regex"),
        Regex::new(r#"data-storemapper-token\s*=\s*["']([^"']+)["']"#).expect("valid regex"),
        Regex::new(r#"token["'\s:=]+([A-Za-z0-9_-]{8,})"#).expect("valid regex"),
    ]
});

static USER_ID_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r#"data-storemapper-id\s*=\s*["']([0-9]+)["']"#).expect("valid regex"),
        Regex::new(r"api/users/([0-9]+)\.js").expect("valid regex"),
    ]
});

/// Extract the Storemapper API token from HTML.
pub(in crate::locator) fn extract_storemapper_token(html: &str) -> Option<String> {
    if !html.contains("storemapper") {
        return None;
    }

    for re in TOKEN_PATTERNS.iter() {
        if let Some(cap) = re.captures(html) {
            if let Some(m) = cap.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }
    None
}

/// Extract the Storemapper user ID from HTML.
pub(in crate::locator) fn extract_storemapper_user_id(html: &str) -> Option<String> {
    if !html.contains("storemapper") {
        return None;
    }

    for re in USER_ID_PATTERNS.iter() {
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
    client: &reqwest::Client,
    token: &str,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let url = format!("https://storemapper.co/api/stores?token={token}");
    let data = crate::locator::fetch::fetch_json(client, &url, user_agent).await?;

    Ok(map_storemapper_stores(&data))
}

/// Fetch stores from Storemapper's user-ID JSONP endpoint.
pub(in crate::locator) async fn fetch_storemapper_stores_by_user_id(
    client: &reqwest::Client,
    user_id: &str,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let url = format!(
        "https://storemapper-herokuapp-com.global.ssl.fastly.net/api/users/{user_id}/stores.js?callback=SMcallback2"
    );
    let body = crate::locator::fetch::fetch_text(client, &url, user_agent).await?;
    let Some(json) = extract_jsonp_payload(&body) else {
        return Ok(vec![]);
    };
    let data: serde_json::Value = serde_json::from_str(json)?;
    Ok(map_storemapper_stores(&data))
}

fn map_storemapper_stores(data: &serde_json::Value) -> Vec<RawStoreLocation> {
    let Some(stores) = data.get("stores").and_then(serde_json::Value::as_array) else {
        return vec![];
    };

    stores
        .iter()
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
                    .and_then(value_as_f64),
                longitude: store
                    .get("lng")
                    .or_else(|| store.get("longitude"))
                    .and_then(value_as_f64),
                phone: store
                    .get("phone")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                locator_source: "storemapper".to_string(),
                raw_data: store.clone(),
            })
        })
        .collect()
}

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
}

fn extract_jsonp_payload(body: &str) -> Option<&str> {
    let open = body.find('(')?;
    let close = body.rfind(')')?;
    if close <= open {
        return None;
    }
    Some(body[open + 1..close].trim())
}

#[cfg(test)]
mod tests {
    use super::{extract_jsonp_payload, extract_storemapper_user_id};

    #[test]
    fn extracts_storemapper_user_id_from_data_attribute() {
        let html = r#"<script data-storemapper-id='8676'></script>"#;
        assert_eq!(extract_storemapper_user_id(html).as_deref(), Some("8676"));
    }

    #[test]
    fn extracts_jsonp_payload() {
        let body = r#"SMcallback2({"stores":[{"id":1,"name":"A"}]});"#;
        assert_eq!(
            extract_jsonp_payload(body),
            Some(r#"{"stores":[{"id":1,"name":"A"}]}"#)
        );
    }
}
