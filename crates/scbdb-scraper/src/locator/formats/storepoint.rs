//! Strategy 4: Storepoint widget extraction.

use regex::Regex;

use crate::locator::types::{LocatorError, RawStoreLocation};

/// Extract the Storepoint widget ID from HTML.
///
/// Recognises patterns such as:
/// - `new StorepointWidget('1682cd22fcf354', ...)`
/// - `api.storepoint.co/v2/<widget_id>/locations`
pub(in crate::locator) fn extract_storepoint_widget_id(html: &str) -> Option<String> {
    if !html.to_ascii_lowercase().contains("storepoint") {
        return None;
    }

    let patterns = [
        r#"StorepointWidget\(\s*['"]([A-Za-z0-9]+)['"]"#,
        r#"StorepointWidget\((?:\\[nrt]|\\u[0-9a-fA-F]{4}|\s)*['"]([A-Za-z0-9]+)['"]"#,
        r"api\.storepoint\.co/v2/([A-Za-z0-9]+)/locations",
        r"widget\.storepoint\.co/([A-Za-z0-9]+)",
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

/// Fetch stores from the Storepoint API and map them to `RawStoreLocation`.
pub(in crate::locator) async fn fetch_storepoint_stores(
    client: &reqwest::Client,
    widget_id: &str,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let url = format!("https://api.storepoint.co/v2/{widget_id}/locations");
    let data = crate::locator::fetch::fetch_json(client, &url, user_agent).await?;

    let Some(stores) = data
        .get("results")
        .and_then(|v| v.get("locations"))
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(vec![]);
    };

    let locations = stores
        .iter()
        .filter_map(|store| {
            let name = store.get("name")?.as_str()?.trim().to_string();
            if name.is_empty() {
                return None;
            }

            let address_line1 = store
                .get("streetaddress")
                .or_else(|| store.get("address"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);

            // Use explicit API fields when available; fall back to address parsing.
            let explicit_country = store
                .get("country")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);

            let (city, state, zip, country) = if explicit_country.is_some() {
                let parsed = address_line1
                    .as_deref()
                    .map_or((None, None, None, None), |addr| {
                        parse_storepoint_address_tail_with_country(addr)
                    });
                (parsed.0, parsed.1, parsed.2, explicit_country)
            } else {
                address_line1
                    .as_deref()
                    .map_or((None, None, None, None), parse_storepoint_address_tail)
            };

            Some(RawStoreLocation {
                external_id: store.get("id").and_then(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .or_else(|| Some(v.to_string()))
                }),
                name,
                address_line1,
                city,
                state,
                zip,
                country,
                latitude: store
                    .get("loc_lat")
                    .and_then(serde_json::Value::as_f64)
                    .or_else(|| {
                        store
                            .get("loc_lat")
                            .and_then(serde_json::Value::as_str)
                            .and_then(|s| s.parse::<f64>().ok())
                    }),
                longitude: store
                    .get("loc_long")
                    .and_then(serde_json::Value::as_f64)
                    .or_else(|| {
                        store
                            .get("loc_long")
                            .and_then(serde_json::Value::as_str)
                            .and_then(|s| s.parse::<f64>().ok())
                    }),
                phone: store
                    .get("phone")
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string),
                locator_source: "storepoint".to_string(),
                raw_data: store.clone(),
            })
        })
        .collect();

    Ok(locations)
}

fn parse_storepoint_address_tail(
    address: &str,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let parts: Vec<&str> = address
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    let country = parts
        .last()
        .map(std::string::ToString::to_string)
        .filter(|s| !s.is_empty());

    let city_state_zip_segment = if parts.len() >= 2 {
        parts.get(parts.len() - 2).copied().unwrap_or("")
    } else {
        ""
    };

    let tokens: Vec<&str> = city_state_zip_segment
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.len() < 3 {
        return (None, None, None, country);
    }

    let zip_candidate = tokens[tokens.len() - 1];
    let state_candidate = tokens[tokens.len() - 2];

    let zip_ok = zip_candidate
        .chars()
        .all(|c| c.is_ascii_digit() || c == '-')
        && zip_candidate.chars().any(|c| c.is_ascii_digit());

    let state_ok =
        state_candidate.len() == 2 && state_candidate.chars().all(|c| c.is_ascii_alphabetic());

    if !zip_ok || !state_ok {
        return (None, None, None, country);
    }

    let city = tokens[..tokens.len() - 2].join(" ");
    let city = if city.is_empty() { None } else { Some(city) };

    (
        city,
        Some(state_candidate.to_string()),
        Some(zip_candidate.to_string()),
        country,
    )
}

/// Parse city/state/zip from an address string when country is already known.
fn parse_storepoint_address_tail_with_country(
    address: &str,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let parts: Vec<&str> = address
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    let mut city_state_zip_segment = parts.last().copied().unwrap_or("");

    let mut tokens: Vec<&str> = city_state_zip_segment
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.len() < 3 && parts.len() >= 2 {
        city_state_zip_segment = parts.get(parts.len() - 2).copied().unwrap_or("");
        tokens = city_state_zip_segment
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .collect();
    }

    if tokens.len() < 3 {
        return (None, None, None, None);
    }

    let zip_candidate = tokens[tokens.len() - 1];
    let state_candidate = tokens[tokens.len() - 2];

    let zip_ok = zip_candidate
        .chars()
        .all(|c| c.is_ascii_digit() || c == '-')
        && zip_candidate.chars().any(|c| c.is_ascii_digit());

    let state_ok =
        state_candidate.len() == 2 && state_candidate.chars().all(|c| c.is_ascii_alphabetic());

    if !zip_ok || !state_ok {
        return (None, None, None, None);
    }

    let city = tokens[..tokens.len() - 2].join(" ");
    let city = if city.is_empty() { None } else { Some(city) };

    (
        city,
        Some(state_candidate.to_string()),
        Some(zip_candidate.to_string()),
        None, // country handled by caller
    )
}

#[cfg(test)]
mod tests {
    use super::{extract_storepoint_widget_id, parse_storepoint_address_tail};

    #[test]
    fn extracts_widget_id_from_constructor() {
        let html = r#"<script>new StorepointWidget('1682cd22fcf354', {selector:'#map'})</script>"#;
        assert_eq!(
            extract_storepoint_widget_id(html).as_deref(),
            Some("1682cd22fcf354")
        );
    }

    #[test]
    fn extracts_widget_id_from_escaped_constructor() {
        let html =
            r#"<script>"new StorepointWidget(\n'1682cd22fcf354', '#storepoint-widget')"</script>"#;
        assert_eq!(
            extract_storepoint_widget_id(html).as_deref(),
            Some("1682cd22fcf354")
        );
    }

    #[test]
    fn parses_city_state_zip_from_tail() {
        let (city, state, zip, country) =
            parse_storepoint_address_tail("1324 5th Street, Jellico TN 37762, USA");

        assert_eq!(city.as_deref(), Some("Jellico"));
        assert_eq!(state.as_deref(), Some("TN"));
        assert_eq!(zip.as_deref(), Some("37762"));
        assert_eq!(country.as_deref(), Some("USA"));
    }
}
