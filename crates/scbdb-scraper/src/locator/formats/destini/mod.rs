//! `Destini` / `lets.shop` locator extraction.

mod parse;

use regex::Regex;

use parse::extract_script_urls_for_destini_probe;
pub(in crate::locator) use parse::fetch_destini_stores;

pub(in crate::locator) const DEFAULT_LATITUDE: f64 = 39.828_175;
pub(in crate::locator) const DEFAULT_LONGITUDE: f64 = -98.579_5;
pub(in crate::locator) const DEFAULT_DISTANCE_MILES: u64 = 100;
pub(in crate::locator) const DEFAULT_MAX_STORES: u64 = 100;
pub(in crate::locator) const DEFAULT_TEXT_STYLE_BM: &str = "RESPECTCASINGPASSED";
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        extract_destini_locator_config,
        parse::{
            extract_script_urls_for_destini_probe, parse_knox_locations,
            parse_product_ids_from_categories,
        },
    };

    #[test]
    fn extracts_destini_config_from_widget_attributes() {
        let html = r#"
            <div id="destini-locator"
                 locator-id="3731"
                 alpha-code="E93"
                 client-id="recessocl">
            </div>
        "#;

        let config = extract_destini_locator_config(html).expect("should extract config");
        assert_eq!(config.alpha_code, "E93");
        assert_eq!(config.locator_id, "3731");
        assert_eq!(config.client_id.as_deref(), Some("recessocl"));
    }

    #[test]
    fn extracts_destini_config_from_json_path_url() {
        let html = r#"
            <script>
                fetch('https://lets.shop/locators/E93/3731/3731.json')
            </script>
        "#;

        let config = extract_destini_locator_config(html).expect("should extract config");
        assert_eq!(config.alpha_code, "E93");
        assert_eq!(config.locator_id, "3731");
    }

    #[test]
    fn returns_none_when_no_destini_markers() {
        let html = "<html><body>No locator here</body></html>";
        assert!(extract_destini_locator_config(html).is_none());
    }

    #[test]
    fn parses_knox_locations_from_response() {
        let response = json!({
            "data": [
                {
                    "id": "store-1",
                    "name": "CVS Pharmacy",
                    "address": "123 Main St",
                    "city": "Columbia",
                    "state": "SC",
                    "postalCode": "29229",
                    "latitude": 34.1405,
                    "longitude": -80.9388
                }
            ]
        });

        let locations = parse_knox_locations(&response);
        assert_eq!(locations.len(), 1);
        let location = &locations[0];

        assert_eq!(location.external_id.as_deref(), Some("store-1"));
        assert_eq!(location.name, "CVS Pharmacy");
        assert_eq!(location.city.as_deref(), Some("Columbia"));
        assert_eq!(location.state.as_deref(), Some("SC"));
        assert_eq!(location.zip.as_deref(), Some("29229"));
        assert_eq!(location.latitude, Some(34.1405));
        assert_eq!(location.longitude, Some(-80.9388));
        assert_eq!(location.locator_source, "destini");
    }

    #[test]
    fn parses_product_ids_from_categories() {
        let response = json!({
            "categories": [
                {
                    "subCategories": [
                        {
                            "products": [
                                {"pID": "prod-1"},
                                {"pID": "prod-2"},
                                {"productId": "prod-3"}
                            ]
                        }
                    ]
                }
            ]
        });

        let mut ids = parse_product_ids_from_categories(&response);
        ids.sort();
        assert_eq!(ids, vec!["prod-1", "prod-2", "prod-3"]);
    }

    #[test]
    fn resolves_relative_nuxt_script_urls_for_probe() {
        let html = r#"
            <script src="/_nuxt/BKlIrEFJ.js"></script>
            <script src="/_nuxt/CK0A3v-F.js"></script>
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
