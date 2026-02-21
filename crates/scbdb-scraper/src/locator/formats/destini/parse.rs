//! `Destini` API fetch orchestration.
//!
//! Iterates a CONUS geographic grid and deduplicates results by coordinate
//! fingerprint. Response parsing lives in [`super::response`].

use std::collections::HashMap;
use std::time::Duration;

use crate::locator::types::{LocatorError, RawStoreLocation};
use crate::locator::{generate_grid, GridConfig};

use super::response::{parse_knox_locations, parse_product_ids_from_categories, value_as_string};
use super::{
    DestiniLocatorConfig, DEFAULT_DISTANCE_MILES, DEFAULT_MAX_STORES, DEFAULT_TEXT_STYLE_BM,
};

/// Coordinate fingerprint for deduplication: 4-decimal lat,lng.
fn coord_key(lat: Option<f64>, lng: Option<f64>) -> String {
    match (lat, lng) {
        (Some(la), Some(lo)) => format!("{la:.4},{lo:.4}"),
        _ => format!("{lat:?},{lng:?}"),
    }
}

/// Deduplicate a vec of locations by coordinate key. First occurrence wins.
fn dedup_by_coordinates(locs: Vec<RawStoreLocation>) -> Vec<RawStoreLocation> {
    let mut seen: HashMap<String, RawStoreLocation> = HashMap::new();
    for loc in locs {
        let key = coord_key(loc.latitude, loc.longitude);
        seen.entry(key).or_insert(loc);
    }
    seen.into_values().collect()
}

/// Single Knox API call for one lat/lng center.
async fn fetch_knox_for_point(
    client: &reqwest::Client,
    user_agent: &str,
    knox_base: &str,
    client_id: &str,
    lat: f64,
    lng: f64,
    distance: u64,
    max_stores: u64,
    text_style_bm: &str,
    product_ids: &[String],
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let knox_url = join_url(knox_base, "knox");
    let payload = serde_json::json!({
        "params": {
            "distance": distance,
            "products": product_ids,
            "latitude": lat,
            "longitude": lng,
            "client": client_id,
            "maxStores": max_stores,
            "textStyleBm": text_style_bm,
        }
    });

    let response = client
        .post(knox_url)
        .header(reqwest::header::USER_AGENT, user_agent)
        .json(&payload)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(parse_knox_locations(&response))
}

/// Fetch store locations from `Destini` using the provider's own bootstrap and
/// API contract.
///
/// Iterates a CONUS geographic grid (~140 points at 200-mile spacing) and
/// deduplicates results by coordinate fingerprint so overlapping radius
/// windows do not produce duplicate entries.
pub(in crate::locator) async fn fetch_destini_stores(
    config: &DestiniLocatorConfig,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let bootstrap_url = format!(
        "https://lets.shop/locators/{}/{}/{}.json",
        config.alpha_code, config.locator_id, config.locator_id
    );

    let bootstrap =
        crate::locator::fetch::fetch_json(&bootstrap_url, timeout_secs, user_agent).await?;
    let context = bootstrap.get("context").unwrap_or(&serde_json::Value::Null);

    let client_id = config
        .client_id
        .clone()
        .or_else(|| value_as_string(context.get("clientId")?))
        .unwrap_or_default();
    if client_id.is_empty() {
        return Ok(vec![]);
    }

    let knox_base = context
        .get("knoxUrl")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("https://hlc7l6v5w6.execute-api.us-west-2.amazonaws.com/prod/");

    let settings = context.get("settings").unwrap_or(&serde_json::Value::Null);

    let distance = settings
        .get("radius")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(DEFAULT_DISTANCE_MILES);
    let max_stores = settings
        .get("maxStores")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(DEFAULT_MAX_STORES);
    let text_style_bm = settings
        .get("textStyleBm")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(DEFAULT_TEXT_STYLE_BM);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;

    let product_ids = fetch_product_ids(&client, user_agent, knox_base, &client_id).await?;
    if product_ids.is_empty() {
        return Ok(vec![]);
    }

    let grid = generate_grid(&GridConfig::conus_coarse()); // ~140 points
    let mut seen: HashMap<String, RawStoreLocation> = HashMap::new();

    for point in &grid {
        let locs = fetch_knox_for_point(
            &client,
            user_agent,
            knox_base,
            &client_id,
            point.lat,
            point.lng,
            distance,
            max_stores,
            text_style_bm,
            &product_ids,
        )
        .await?;

        if locs.len() as u64 >= max_stores {
            tracing::warn!(
                lat = point.lat,
                lng = point.lng,
                max_stores,
                "Knox returned max_stores; locations in this region may be truncated"
            );
        }

        for loc in locs {
            let key = coord_key(loc.latitude, loc.longitude);
            seen.entry(key).or_insert(loc);
        }

        // Courtesy delay — 140 calls at 500ms ≈ 70 s total per brand
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(seen.into_values().collect())
}

async fn fetch_product_ids(
    client: &reqwest::Client,
    user_agent: &str,
    knox_base: &str,
    client_id: &str,
) -> Result<Vec<String>, LocatorError> {
    let url = join_url(knox_base, "productCategories");
    let payload = serde_json::json!({
        "params": {
            "categoryIds": "",
            "subCategoryIds": "",
            "clientId": client_id,
            "level": 2,
        }
    });

    let response = client
        .post(url)
        .header(reqwest::header::USER_AGENT, user_agent)
        .json(&payload)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(parse_product_ids_from_categories(&response))
}

fn join_url(base: &str, path: &str) -> String {
    format!("{}{path}", base.trim_end_matches('/').to_string() + "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locator::types::RawStoreLocation;

    fn make_loc(lat: f64, lng: f64) -> RawStoreLocation {
        RawStoreLocation {
            external_id: None,
            name: "Store A".to_string(),
            address_line1: None,
            city: Some("Columbia".to_string()),
            state: Some("SC".to_string()),
            zip: None,
            country: None,
            latitude: Some(lat),
            longitude: Some(lng),
            phone: None,
            locator_source: "destini".to_string(),
            raw_data: serde_json::Value::Null,
        }
    }

    #[test]
    fn deduplicates_overlapping_destini_results() {
        // 33.12340 and 33.12341 both format to "33.1234" with {:.4} → same dedup key
        let locs = vec![
            make_loc(33.12340, -80.12340),
            make_loc(33.12341, -80.12340), // duplicate — rounds to same key
            make_loc(34.00000, -81.00000), // distinct
        ];
        let deduped = dedup_by_coordinates(locs);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn coord_key_formats_to_four_decimals() {
        assert_eq!(coord_key(Some(33.1234), Some(-80.9999)), "33.1234,-80.9999");
    }

    #[test]
    fn coord_key_handles_none() {
        let key = coord_key(None, None);
        assert!(key.contains("None"));
    }
}
