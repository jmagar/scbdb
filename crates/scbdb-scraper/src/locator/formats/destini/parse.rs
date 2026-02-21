//! `Destini` API fetch and response parsing.

use regex::Regex;

use crate::locator::types::{LocatorError, RawStoreLocation};

use super::{
    DestiniLocatorConfig, DEFAULT_DISTANCE_MILES, DEFAULT_LATITUDE, DEFAULT_LONGITUDE,
    DEFAULT_MAX_STORES, DEFAULT_TEXT_STYLE_BM,
};

/// Fetch store locations from `Destini` using the provider's own bootstrap and
/// API contract.
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
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let product_ids = fetch_product_ids(&client, user_agent, knox_base, &client_id).await?;
    if product_ids.is_empty() {
        return Ok(vec![]);
    }

    let knox_url = join_url(knox_base, "knox");
    let payload = serde_json::json!({
        "params": {
            "distance": distance,
            "products": product_ids,
            "latitude": DEFAULT_LATITUDE,
            "longitude": DEFAULT_LONGITUDE,
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

pub(super) fn parse_product_ids_from_categories(response: &serde_json::Value) -> Vec<String> {
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    let categories = response
        .get("categories")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    for category in categories {
        let sub_categories = category
            .get("subCategories")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();

        for sub_category in sub_categories {
            let products = sub_category
                .get("products")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();

            for product in products {
                if let Some(product_id) = product
                    .get("pID")
                    .or_else(|| product.get("productId"))
                    .and_then(value_as_string)
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                {
                    seen.insert(product_id);
                }
            }
        }
    }

    seen.into_iter().collect()
}

pub(super) fn parse_knox_locations(response: &serde_json::Value) -> Vec<RawStoreLocation> {
    response
        .get("data")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flat_map(|stores| stores.iter())
        .filter_map(map_knox_store)
        .collect()
}

fn map_knox_store(store: &serde_json::Value) -> Option<RawStoreLocation> {
    let name = store
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)?;

    Some(RawStoreLocation {
        external_id: store.get("id").and_then(value_as_string),
        name,
        address_line1: store
            .get("address")
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
            .get("postalCode")
            .or_else(|| store.get("zip"))
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
        latitude: store.get("latitude").and_then(value_as_f64),
        longitude: store.get("longitude").and_then(value_as_f64),
        phone: store
            .get("phone")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        locator_source: "destini".to_string(),
        raw_data: store.clone(),
    })
}

fn join_url(base: &str, path: &str) -> String {
    format!("{}{path}", base.trim_end_matches('/').to_string() + "/")
}

pub(super) fn extract_script_urls_for_destini_probe(html: &str, locator_url: &str) -> Vec<String> {
    let script_src_re = Regex::new(r#"<script[^>]+src\s*=\s*["']([^"']+\.js[^"']*)["'][^>]*>"#)
        .expect("valid regex");
    let link_href_re = Regex::new(r#"<link[^>]+href\s*=\s*["']([^"']+\.js[^"']*)["'][^>]*>"#)
        .expect("valid regex");
    let base_url = reqwest::Url::parse(locator_url).ok();

    let mut urls = Vec::new();

    for regex in [&script_src_re, &link_href_re] {
        for captures in regex.captures_iter(html) {
            let Some(source) = captures.get(1).map(|m| m.as_str().trim()) else {
                continue;
            };

            let resolved = if source.starts_with("http://") || source.starts_with("https://") {
                Some(source.to_string())
            } else {
                base_url
                    .as_ref()
                    .and_then(|base| base.join(source).ok())
                    .map(|url| url.to_string())
            };

            let Some(url) = resolved else {
                continue;
            };

            let lowered = url.to_ascii_lowercase();
            if lowered.contains("/_nuxt/")
                || lowered.contains("locator")
                || lowered.contains("where-to-buy")
                || lowered.contains("lets.shop")
            {
                urls.push(url);
            }
        }
    }

    // Preserve script order but deduplicate.
    let mut seen = std::collections::BTreeSet::new();
    urls.into_iter()
        .filter(|url| seen.insert(url.clone()))
        .collect()
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

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
}
