//! Strategy 5: Roseperl/Secomapp Shopify store-locator extraction.

use regex::Regex;

use crate::locator::fetch::fetch_text;
use crate::locator::types::{LocatorError, RawStoreLocation};

/// Extract a Roseperl "where to buy" JS URL from HTML.
///
/// Recognises links such as:
/// - `https://cdn.roseperl.com/storelocator-prod/wtb/<id>.js?shop=...`
pub(in crate::locator) fn extract_roseperl_wtb_url(html: &str) -> Option<String> {
    let normalized = html.replace("\\/", "/");
    let re = Regex::new(r#"https://cdn\.roseperl\.com/storelocator-prod/wtb/[^"'\s]+"#)
        .expect("valid regex");
    re.find(&normalized).map(|m| {
        m.as_str()
            .trim_end_matches('\\')
            .trim_end_matches('"')
            .trim_end_matches('\'')
            .to_string()
    })
}

/// Fetch stores from a Roseperl WTB JS endpoint and map them to
/// `RawStoreLocation`.
pub(in crate::locator) async fn fetch_roseperl_stores(
    wtb_url: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let body = fetch_text(wtb_url, timeout_secs, user_agent).await?;
    let Some(payload) = extract_assignment_payload(&body, "SCASLWtb") else {
        return Ok(vec![]);
    };
    let data: serde_json::Value = serde_json::from_str(payload)?;

    let Some(stores) = data.get("locations").and_then(serde_json::Value::as_array) else {
        return Ok(vec![]);
    };

    let locations = stores
        .iter()
        .filter_map(|store| {
            let name = store
                .get("title")
                .or_else(|| store.get("name"))
                .and_then(serde_json::Value::as_str)?
                .trim()
                .to_string();
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
                    .get("address")
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string),
                city: store
                    .get("city")
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string),
                state: store
                    .get("state")
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string),
                zip: store
                    .get("zipcode")
                    .or_else(|| store.get("zip"))
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string),
                country: store
                    .get("country")
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string),
                latitude: store
                    .get("latitude")
                    .and_then(value_as_f64)
                    .or_else(|| store.get("lat").and_then(value_as_f64)),
                longitude: store
                    .get("longitude")
                    .and_then(value_as_f64)
                    .or_else(|| store.get("lng").and_then(value_as_f64)),
                phone: store
                    .get("phone")
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string),
                locator_source: "roseperl".to_string(),
                raw_data: store.clone(),
            })
        })
        .collect();

    Ok(locations)
}

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
}

fn extract_assignment_payload<'a>(js: &'a str, variable: &str) -> Option<&'a str> {
    let needle = format!("{variable}=");
    let start = js.find(&needle)? + needle.len();
    let remainder = &js[start..];
    let open_offset = remainder.find('{')?;
    let chars = remainder[open_offset..].char_indices();

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut begin = None;

    for (idx, ch) in chars {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    begin = Some(idx);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    let begin_idx = begin?;
                    let end_idx = idx + 1;
                    return Some(&remainder[open_offset + begin_idx..open_offset + end_idx]);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{extract_assignment_payload, extract_roseperl_wtb_url};

    #[test]
    fn extracts_wtb_url_from_escaped_html() {
        let html = r#"<script>var urls=[\"https:\/\/cdn.roseperl.com\/storelocator-prod\/wtb\/abc.js?shop=x.myshopify.com\"];</script>"#;
        assert_eq!(
            extract_roseperl_wtb_url(html).as_deref(),
            Some("https://cdn.roseperl.com/storelocator-prod/wtb/abc.js?shop=x.myshopify.com")
        );
    }

    #[test]
    fn extracts_assignment_payload() {
        let js = r#"SCASLWtb={"locations":[{"id":1,"title":"A"}]};foo=1;"#;
        assert_eq!(
            extract_assignment_payload(js, "SCASLWtb"),
            Some(r#"{"locations":[{"id":1,"title":"A"}]}"#)
        );
    }
}
