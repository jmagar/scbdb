//! `AskHoodie` embed extraction.

use regex::Regex;

use crate::locator::types::{LocatorError, RawStoreLocation};

const ASKHOODIE_SEARCH_URL: &str = "https://www.askhoodie.com/api/search";
const ASKHOODIE_INDEX_PRODUCTS_V2: &str = "all_PRODUCTS_V2";

/// Extract `AskHoodie` embed ID from HTML.
pub(in crate::locator) fn extract_askhoodie_embed_id(html: &str) -> Option<String> {
    if !html.contains("askhoodie") {
        return None;
    }

    let pattern =
        Regex::new(r#"hoodieEmbedWtbV2\(\s*\"([0-9a-fA-F-]{36})\""#).expect("valid regex");
    pattern
        .captures(html)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

/// Fetch locations from `AskHoodie` using the production embed search contract.
///
/// Production behavior (verified 2026-02-21):
/// - endpoint: `POST /api/search`
/// - payload: `{ embedToken, method: "search", args: [[{ indexName, query, params }]] }`
/// - response: `{ results: [{ hits: [...] }] }`
pub(in crate::locator) async fn fetch_askhoodie_stores(
    client: &reqwest::Client,
    embed_id: &str,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let embed_token = format!("{embed_id}__dummy");

    // Query a broad radius around a few US hubs to maximize coverage.
    let search_centers = [
        (39.8283_f64, -98.5795_f64),
        (44.9778, -93.2650),
        (34.0522, -118.2437),
        (40.7128, -74.0060),
        (29.7604, -95.3698),
    ];

    let mut dedup: std::collections::HashMap<String, RawStoreLocation> =
        std::collections::HashMap::new();

    for (lat, lng) in search_centers {
        search_center(client, user_agent, &embed_token, lat, lng, &mut dedup).await?;
    }

    if dedup.is_empty() {
        // Clear fallback when AskHoodie responds but no store results are available.
        return Ok(vec![]);
    }

    Ok(dedup.into_values().collect())
}

async fn search_center(
    client: &reqwest::Client,
    user_agent: &str,
    embed_token: &str,
    lat: f64,
    lng: f64,
    dedup: &mut std::collections::HashMap<String, RawStoreLocation>,
) -> Result<(), crate::locator::types::LocatorError> {
    let mut page = 0_u64;
    let mut pages_without_hits = 0_u8;

    loop {
        let body = build_search_payload(embed_token, lat, lng, page);

        let response = client
            .post(ASKHOODIE_SEARCH_URL)
            .header(reqwest::header::USER_AGENT, user_agent)
            .json(&body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let hits = extract_hits_array(&response);
        let Some(hits) = hits else {
            return Ok(());
        };

        if hits.is_empty() {
            pages_without_hits = pages_without_hits.saturating_add(1);
            if pages_without_hits >= 2 {
                return Ok(());
            }
            page = page.saturating_add(1);
            if page >= 4 {
                return Ok(());
            }
            continue;
        }

        pages_without_hits = 0;

        for hit in hits {
            let Some(external_id) = extract_external_id(hit) else {
                continue;
            };

            if dedup.contains_key(external_id) {
                continue;
            }

            if let Some(location) = build_location_from_hit(hit, external_id) {
                dedup.insert(external_id.to_string(), location);
            }
        }

        let (next_page, has_next_page) = next_page_state(&response);
        if !has_next_page {
            return Ok(());
        }
        page = next_page;
    }
}

fn build_search_payload(embed_token: &str, lat: f64, lng: f64, page: u64) -> serde_json::Value {
    serde_json::json!({
        "embedToken": embed_token,
        "method": "search",
        "args": [[{
            "indexName": ASKHOODIE_INDEX_PRODUCTS_V2,
            "query": "",
            "params": {
                "aroundLatLng": format!("{lat},{lng}"),
                "aroundRadius": 2_500_000,
                "hitsPerPage": 1000,
                "page": page,
                "attributesToRetrieve": [
                    "MASTER_D_ID",
                    "MASTER_D_NAME",
                    "MASTER_D_ADDRESS",
                    "MASTER_D_CITY",
                    "MASTER_D_STATE",
                    "MASTER_D_ZIP",
                    "MASTER_D_COUNTRY",
                    "MASTER_D_PHONE",
                    "DISPENSARY_NAME",
                    "FULL_ADDRESS",
                    "D_CITY",
                    "D_STATE",
                    "D_ZIP",
                    "D_COUNTRY",
                    "PHONE",
                    "_geoloc"
                ]
            }
        }]]
    })
}

fn extract_hits_array(response: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    response
        .get("results")
        .and_then(serde_json::Value::as_array)
        .and_then(|results| results.first())
        .and_then(|first| first.get("hits"))
        .and_then(serde_json::Value::as_array)
        .or_else(|| response.get("hits").and_then(serde_json::Value::as_array))
}

fn next_page_state(response: &serde_json::Value) -> (u64, bool) {
    let first_result = response
        .get("results")
        .and_then(serde_json::Value::as_array)
        .and_then(|results| results.first())
        .unwrap_or(response);

    let page = first_result
        .get("page")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    let nb_pages = first_result
        .get("nbPages")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    let next_page = page.saturating_add(1);
    let has_next_page = nb_pages > 0 && next_page < nb_pages && next_page <= 25;

    (next_page, has_next_page)
}

fn extract_external_id(hit: &serde_json::Value) -> Option<&str> {
    hit.get("MASTER_D_ID")
        .or_else(|| hit.get("DISPENSARY_ID"))
        .or_else(|| hit.get("objectID"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn build_location_from_hit(hit: &serde_json::Value, external_id: &str) -> Option<RawStoreLocation> {
    let name = hit
        .get("MASTER_D_NAME")
        .or_else(|| hit.get("DISPENSARY_NAME"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)?;

    let latitude = hit
        .get("_geoloc")
        .and_then(|g| g.get("lat"))
        .and_then(value_as_f64);
    let longitude = hit
        .get("_geoloc")
        .and_then(|g| g.get("lng"))
        .and_then(value_as_f64);

    Some(RawStoreLocation {
        external_id: Some(external_id.to_string()),
        name,
        address_line1: hit
            .get("MASTER_D_ADDRESS")
            .or_else(|| hit.get("FULL_ADDRESS"))
            .or_else(|| hit.get("address"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        city: hit
            .get("MASTER_D_CITY")
            .or_else(|| hit.get("D_CITY"))
            .or_else(|| hit.get("city"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        state: hit
            .get("MASTER_D_STATE")
            .or_else(|| hit.get("D_STATE"))
            .or_else(|| hit.get("state"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        zip: hit
            .get("MASTER_D_ZIP")
            .or_else(|| hit.get("D_ZIP"))
            .or_else(|| hit.get("zip"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        country: hit
            .get("MASTER_D_COUNTRY")
            .or_else(|| hit.get("D_COUNTRY"))
            .or_else(|| hit.get("country"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        latitude,
        longitude,
        phone: hit
            .get("MASTER_D_PHONE")
            .or_else(|| hit.get("PHONE"))
            .or_else(|| hit.get("phone"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        locator_source: "askhoodie".to_string(),
        raw_data: hit.clone(),
    })
}

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
}

#[cfg(test)]
mod tests {
    use super::{extract_askhoodie_embed_id, extract_hits_array, next_page_state, value_as_f64};

    #[test]
    fn extracts_embed_id_from_script_call() {
        let html = r#"<script>document.cookie = hoodieEmbedWtbV2("37ce33f1-f401-4a43-9f0e-8a62cf15272f","askhoodieDiv",document.cookie);</script>"#;
        assert_eq!(
            extract_askhoodie_embed_id(html).as_deref(),
            Some("37ce33f1-f401-4a43-9f0e-8a62cf15272f")
        );
    }

    #[test]
    fn extracts_hits_from_wrapped_results_payload() {
        let payload = serde_json::json!({
            "results": [
                {
                    "hits": [
                        { "MASTER_D_ID": "123", "MASTER_D_NAME": "Store A" }
                    ],
                    "page": 0,
                    "nbPages": 2
                }
            ]
        });

        let hits = extract_hits_array(&payload).expect("hits expected");
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].get("MASTER_D_ID").and_then(|v| v.as_str()),
            Some("123")
        );
    }

    #[test]
    fn next_page_state_handles_missing_fields_defensively() {
        let payload = serde_json::json!({"results": [{"hits": []}]});
        let (next_page, has_next_page) = next_page_state(&payload);
        assert_eq!(next_page, 1);
        assert!(!has_next_page);
    }

    #[test]
    fn value_as_f64_accepts_numeric_strings() {
        assert_eq!(value_as_f64(&serde_json::json!(42.5)), Some(42.5));
        assert_eq!(value_as_f64(&serde_json::json!("42.5")), Some(42.5));
        assert_eq!(value_as_f64(&serde_json::json!("not-a-number")), None);
    }
}
