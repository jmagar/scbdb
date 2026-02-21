//! `Destini` / `lets.shop` locator extraction.

use regex::Regex;

use crate::locator::types::{LocatorError, RawStoreLocation};

const DEFAULT_LATITUDE: f64 = 39.828_175;
const DEFAULT_LONGITUDE: f64 = -98.579_5;
const DEFAULT_DISTANCE_MILES: u64 = 100;
const DEFAULT_MAX_STORES: u64 = 100;
const DEFAULT_TEXT_STYLE_BM: &str = "RESPECTCASINGPASSED";
const MAX_SCRIPT_PROBES: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::locator) struct DestiniLocatorConfig {
    pub alpha_code: String,
    pub locator_id: String,
    pub client_id: Option<String>,
}

/// Extract `Destini` locator config from page HTML.
///
/// Supported markers:
/// - `<div id="destini-locator" locator-id="3731" alpha-code="E93" client-id="recessocl">`
/// - `https://lets.shop/locators/E93/3731/3731.json`
pub(in crate::locator) fn extract_destini_locator_config(
    html: &str,
) -> Option<DestiniLocatorConfig> {
    if !html.contains("destini-locator") && !html.contains("lets.shop") {
        return None;
    }

    let mut alpha_code = capture_first(html, r#"alpha-code\s*=\s*["']([A-Za-z0-9_-]{1,64})["']"#);
    let mut locator_id = capture_first(html, r#"locator-id\s*=\s*["']([A-Za-z0-9_-]{1,64})["']"#);

    if alpha_code.is_none() || locator_id.is_none() {
        if let Some((url_alpha, url_locator)) = extract_locator_json_path_parts(html) {
            if alpha_code.is_none() {
                alpha_code = Some(url_alpha);
            }
            if locator_id.is_none() {
                locator_id = Some(url_locator);
            }
        }
    }

    let alpha_code = alpha_code?;
    let locator_id = locator_id?;

    let client_id = capture_first(html, r#"client-id\s*=\s*["']([A-Za-z0-9_-]{1,128})["']"#);

    Some(DestiniLocatorConfig {
        alpha_code,
        locator_id,
        client_id,
    })
}

/// Discover `Destini` config from page HTML and linked JS bundles.
///
/// This is needed for modern SPA locator pages that render provider metadata
/// inside route chunks (for example `/_nuxt/*.js`) instead of inline HTML.
pub(in crate::locator) async fn discover_destini_locator_config(
    html: &str,
    locator_url: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Option<DestiniLocatorConfig> {
    if let Some(config) = extract_destini_locator_config(html) {
        return Some(config);
    }

    let script_urls = extract_script_urls_for_destini_probe(html, locator_url);
    for script_url in script_urls.iter().take(MAX_SCRIPT_PROBES) {
        match crate::locator::fetch::fetch_text(script_url, timeout_secs, user_agent).await {
            Ok(script_body) => {
                if let Some(config) = extract_destini_locator_config(&script_body) {
                    return Some(config);
                }
            }
            Err(error) => {
                tracing::debug!(
                    script_url,
                    %error,
                    "failed fetching candidate Destini script"
                );
            }
        }
    }

    None
}

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

fn parse_product_ids_from_categories(response: &serde_json::Value) -> Vec<String> {
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

fn parse_knox_locations(response: &serde_json::Value) -> Vec<RawStoreLocation> {
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

fn capture_first(haystack: &str, pattern: &str) -> Option<String> {
    capture_groups(haystack, pattern, 1)
}

fn capture_groups(haystack: &str, pattern: &str, group: usize) -> Option<String> {
    let regex = Regex::new(pattern).expect("valid regex");
    regex
        .captures(haystack)
        .and_then(|captures| captures.get(group).map(|m| m.as_str().to_string()))
}

fn extract_locator_json_path_parts(html: &str) -> Option<(String, String)> {
    let regex = Regex::new(
        r"lets\.shop/locators/([A-Za-z0-9_-]{1,64})/([A-Za-z0-9_-]{1,64})/([A-Za-z0-9_-]{1,64})\.json",
    )
    .expect("valid regex");

    let captures = regex.captures(html)?;
    let alpha = captures.get(1).map(|m| m.as_str().to_string())?;
    let locator_a = captures.get(2).map(|m| m.as_str().to_string())?;
    let locator_b = captures.get(3).map(|m| m.as_str().to_string())?;

    if locator_a != locator_b {
        return None;
    }

    Some((alpha, locator_a))
}

fn extract_script_urls_for_destini_probe(html: &str, locator_url: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        extract_destini_locator_config, extract_script_urls_for_destini_probe,
        parse_knox_locations, parse_product_ids_from_categories,
    };

    #[test]
    fn extracts_destini_config_from_widget_attributes() {
        let html = r#"
            <div id="destini-locator"
                 locator-id="3731"
                 alpha-code="E93"
                 client-id="recessocl"></div>
        "#;

        let config = extract_destini_locator_config(html).expect("config expected");
        assert_eq!(config.alpha_code, "E93");
        assert_eq!(config.locator_id, "3731");
        assert_eq!(config.client_id.as_deref(), Some("recessocl"));
    }

    #[test]
    fn extracts_destini_config_from_locator_json_url() {
        let html = r#"
            <script>
                const cfg = "https://lets.shop/locators/E93/3731/3731.json";
            </script>
        "#;

        let config = extract_destini_locator_config(html).expect("config expected");
        assert_eq!(config.alpha_code, "E93");
        assert_eq!(config.locator_id, "3731");
        assert_eq!(config.client_id, None);
    }

    #[test]
    fn parses_product_ids_from_categories_response() {
        let payload = json!({
            "categories": [
                {
                    "subCategories": [
                        {
                            "products": [
                                { "pID": "850019179811" },
                                { "pID": "850019179873" },
                                { "pID": "850019179811" }
                            ]
                        }
                    ]
                }
            ]
        });

        let product_ids = parse_product_ids_from_categories(&payload);
        assert_eq!(product_ids.len(), 2);
        assert!(product_ids.iter().any(|id| id == "850019179811"));
        assert!(product_ids.iter().any(|id| id == "850019179873"));
    }

    #[test]
    fn parses_knox_store_records() {
        let payload = json!({
            "data": [
                {
                    "id": "9776",
                    "name": "CVS Pharmacy",
                    "address": "1002 Sams Crossing Dr",
                    "city": "Columbia",
                    "state": "SC",
                    "postalCode": "29229",
                    "latitude": 34.1405,
                    "longitude": -80.9388,
                    "phone": "(803)788-0535"
                }
            ]
        });

        let locations = parse_knox_locations(&payload);
        assert_eq!(locations.len(), 1);

        let location = &locations[0];
        assert_eq!(location.external_id.as_deref(), Some("9776"));
        assert_eq!(location.name, "CVS Pharmacy");
        assert_eq!(location.city.as_deref(), Some("Columbia"));
        assert_eq!(location.state.as_deref(), Some("SC"));
        assert_eq!(location.zip.as_deref(), Some("29229"));
        assert_eq!(location.latitude, Some(34.1405));
        assert_eq!(location.longitude, Some(-80.9388));
        assert_eq!(location.locator_source, "destini");
    }

    #[test]
    fn resolves_relative_nuxt_script_urls_for_probe() {
        let html = r#"
            <link rel="modulepreload" as="script" href="/_nuxt/CK0A3v-F.js">
            <script src="/_nuxt/BKlIrEFJ.js"></script>
            <script src="https://cdn.example.com/vendor.js"></script>
        "#;

        let urls =
            extract_script_urls_for_destini_probe(html, "https://takearecess.com/where-to-buy");
        assert!(urls
            .iter()
            .any(|url| url == "https://takearecess.com/_nuxt/BKlIrEFJ.js"));
        assert!(urls
            .iter()
            .any(|url| url == "https://takearecess.com/_nuxt/CK0A3v-F.js"));
    }
}
