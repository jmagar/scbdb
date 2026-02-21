//! `WordPress` Agile Store Locator extraction.

use regex::Regex;

use crate::locator::types::{LocatorError, RawStoreLocation};

const AGILE_STORE_LOCATOR_ATTEMPTS: usize = 3;
const AGILE_STORE_LOCATOR_BACKOFF_MS: [u64; 3] = [0, 300, 900];

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgileStoreLocatorConfig {
    ajax_url: String,
    nonce: String,
    lang: String,
    load_all: String,
    layout: String,
    stores: Option<String>,
}

/// Extract Agile Store Locator runtime config from HTML.
pub(in crate::locator) fn extract_agile_store_locator_config(
    html: &str,
) -> Option<(String, String, String, String, String, Option<String>)> {
    extract_config(html).map(|cfg| {
        (
            cfg.ajax_url,
            cfg.nonce,
            cfg.lang,
            cfg.load_all,
            cfg.layout,
            cfg.stores,
        )
    })
}

/// Fetch stores from the Agile Store Locator `WordPress` AJAX endpoint.
#[allow(clippy::too_many_arguments)]
pub(in crate::locator) async fn fetch_agile_store_locator_stores(
    ajax_url: &str,
    nonce: &str,
    lang: &str,
    load_all: &str,
    layout: &str,
    stores: Option<&str>,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let mut last_http_error: Option<reqwest::Error> = None;
    let mut last_json_error: Option<serde_json::Error> = None;

    for attempt in 0..AGILE_STORE_LOCATOR_ATTEMPTS {
        if let Some(delay_ms) = AGILE_STORE_LOCATOR_BACKOFF_MS.get(attempt).copied() {
            if delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        let mut query = vec![
            ("action", "asl_load_stores".to_string()),
            ("nonce", nonce.to_string()),
            ("asl_lang", lang.to_string()),
            ("load_all", load_all.to_string()),
            ("layout", layout.to_string()),
        ];

        if let Some(stores_filter) = stores {
            if !stores_filter.trim().is_empty() {
                query.push(("stores", stores_filter.to_string()));
            }
        }

        let response = match client
            .get(ajax_url)
            .header(reqwest::header::USER_AGENT, user_agent)
            .query(&query)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                last_http_error = Some(error);
                if attempt + 1 < AGILE_STORE_LOCATOR_ATTEMPTS {
                    continue;
                }
                break;
            }
        };

        let response = match response.error_for_status() {
            Ok(response) => response,
            Err(error) => {
                last_http_error = Some(error);
                if attempt + 1 < AGILE_STORE_LOCATOR_ATTEMPTS {
                    continue;
                }
                break;
            }
        };
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                last_http_error = Some(error);
                if attempt + 1 < AGILE_STORE_LOCATOR_ATTEMPTS {
                    continue;
                }
                break;
            }
        };

        let parsed = match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(value) => value,
            Err(error) => {
                last_json_error = Some(error);
                if attempt + 1 < AGILE_STORE_LOCATOR_ATTEMPTS {
                    continue;
                }
                break;
            }
        };

        return Ok(parse_agile_store_locator_stores(&parsed));
    }

    if let Some(error) = last_json_error {
        return Err(LocatorError::Json(error));
    }
    if let Some(error) = last_http_error {
        return Err(LocatorError::Http(error));
    }

    Ok(vec![])
}

fn extract_config(html: &str) -> Option<AgileStoreLocatorConfig> {
    if !html.contains("agile-store-locator") && !html.contains("asl_load_stores") {
        return None;
    }

    let remote = extract_json_var(html, "ASL_REMOTE")?;
    let config = extract_json_var(html, "asl_configuration")?;

    let ajax_url = remote
        .get("ajax_url")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)?;

    let nonce = remote
        .get("nonce")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)?;

    let lang = config
        .get("lang")
        .and_then(serde_json::Value::as_str)
        .map_or("", str::trim)
        .to_string();

    let load_all = config
        .get("load_all")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("1")
        .to_string();

    let layout = config
        .get("layout")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("0")
        .to_string();

    let stores = config
        .get("stores")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);

    Some(AgileStoreLocatorConfig {
        ajax_url,
        nonce,
        lang,
        load_all,
        layout,
        stores,
    })
}

fn extract_json_var(html: &str, var_name: &str) -> Option<serde_json::Value> {
    let pattern = format!(r"(?s)var\s+{var_name}\s*=\s*(\{{.*?\}});");
    let re = Regex::new(&pattern).expect("valid regex");
    let captures = re.captures(html)?;
    let object_text = captures.get(1)?.as_str();
    serde_json::from_str(object_text).ok()
}

fn parse_agile_store_locator_stores(payload: &serde_json::Value) -> Vec<RawStoreLocation> {
    let stores = if let Some(arr) = payload.as_array() {
        arr
    } else if let Some(arr) = payload.get("stores").and_then(serde_json::Value::as_array) {
        arr
    } else {
        return vec![];
    };

    stores
        .iter()
        .filter_map(|store| {
            let name = store
                .get("title")
                .or_else(|| store.get("name"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)?;

            Some(RawStoreLocation {
                external_id: store.get("id").and_then(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                }),
                name,
                address_line1: store
                    .get("street")
                    .or_else(|| store.get("address"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
                city: store
                    .get("city")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
                state: store
                    .get("state")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
                zip: store
                    .get("postal_code")
                    .or_else(|| store.get("zip"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
                country: store
                    .get("country")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
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
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
                locator_source: "agile_store_locator".to_string(),
                raw_data: store.clone(),
            })
        })
        .collect()
}

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|v| v.parse::<f64>().ok()))
}

#[cfg(test)]
mod tests {
    use super::{extract_agile_store_locator_config, parse_agile_store_locator_stores};

    #[test]
    fn extracts_agile_store_locator_runtime_config() {
        let html = r#"
            <link rel='stylesheet' id='agile-store-locator-init-css' href='https://example.com/wp-content/plugins/agile-store-locator/public/css/init.css?ver=5.1.1' media='all' />
            <script>
            var ASL_REMOTE = {"ajax_url":"https://example.com/wp-admin/admin-ajax.php","nonce":"abc123","default_lang":"en_US","lang":""};
            var asl_configuration = {"lang":"","load_all":"1","layout":"0","stores":"42,84"};
            </script>
        "#;

        let config = extract_agile_store_locator_config(html);
        assert_eq!(
            config,
            Some((
                "https://example.com/wp-admin/admin-ajax.php".to_string(),
                "abc123".to_string(),
                "".to_string(),
                "1".to_string(),
                "0".to_string(),
                Some("42,84".to_string())
            ))
        );
    }

    #[test]
    fn parses_agile_store_locator_payload_array() {
        let payload = serde_json::json!([
            {
                "id": "962",
                "title": "*CASON SMOKE FOR LESS",
                "street": "517 CASON LN",
                "city": "MURFREESBORO",
                "state": "TN",
                "postal_code": "37127",
                "country": "United States",
                "lat": "35.8416063",
                "lng": "-86.4400538",
                "phone": ""
            },
            {
                "id": "963",
                "title": "  ",
                "street": "ignored"
            }
        ]);

        let stores = parse_agile_store_locator_stores(&payload);
        assert_eq!(stores.len(), 1);

        let first = &stores[0];
        assert_eq!(first.external_id.as_deref(), Some("962"));
        assert_eq!(first.name, "*CASON SMOKE FOR LESS");
        assert_eq!(first.address_line1.as_deref(), Some("517 CASON LN"));
        assert_eq!(first.city.as_deref(), Some("MURFREESBORO"));
        assert_eq!(first.state.as_deref(), Some("TN"));
        assert_eq!(first.zip.as_deref(), Some("37127"));
        assert_eq!(first.country.as_deref(), Some("United States"));
        assert_eq!(first.latitude, Some(35.8416063));
        assert_eq!(first.longitude, Some(-86.4400538));
        assert_eq!(first.phone, None);
        assert_eq!(first.locator_source, "agile_store_locator");
    }

    #[test]
    fn parses_store_array_from_object_wrapper() {
        let payload = serde_json::json!({
            "stores": [
                {
                    "id": 1,
                    "name": "Example Store",
                    "address": "123 Main",
                    "city": "Austin",
                    "state": "TX",
                    "zip": "78701",
                    "country": "US",
                    "latitude": 30.26,
                    "longitude": -97.74,
                    "phone": "555-0100"
                }
            ]
        });

        let stores = parse_agile_store_locator_stores(&payload);
        assert_eq!(stores.len(), 1);
        assert_eq!(stores[0].external_id.as_deref(), Some("1"));
        assert_eq!(stores[0].name, "Example Store");
    }
}
