//! Store locator crawler.
//!
//! Tries extraction strategies in priority order (Locally.com, Storemapper,
//! JSON-LD, embedded JSON) and returns the first successful result.

pub(crate) mod fetch;
mod formats;
pub mod types;

pub use types::{LocatorError, RawStoreLocation};

use fetch::fetch_html;
use formats::{
    extract_json_embed_locations, extract_jsonld_locations, extract_locally_company_id,
    extract_storemapper_token, fetch_locally_stores, fetch_storemapper_stores,
};

/// Fetch store locations from a brand's store locator page.
///
/// Tries extraction strategies in order (Locally.com, Storemapper, JSON-LD,
/// embedded JSON) and returns the first successful result. Returns
/// `Ok(vec![])` when the page is reachable but no locations can be parsed.
///
/// # Errors
///
/// Returns [`LocatorError::Http`] if the locator page cannot be fetched.
pub async fn fetch_store_locations(
    locator_url: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let html = fetch_html(locator_url, timeout_secs, user_agent).await?;

    // Strategy 1: Locally.com widget
    if let Some(company_id) = extract_locally_company_id(&html) {
        tracing::debug!(locator_url, company_id, "detected Locally.com widget");
        let stores = fetch_locally_stores(&company_id, timeout_secs, user_agent).await?;
        if !stores.is_empty() {
            return Ok(stores);
        }
    }

    // Strategy 2: Storemapper widget
    if let Some(token) = extract_storemapper_token(&html) {
        tracing::debug!(locator_url, token, "detected Storemapper widget");
        let stores = fetch_storemapper_stores(&token, timeout_secs, user_agent).await?;
        if !stores.is_empty() {
            return Ok(stores);
        }
    }

    // Strategy 3: schema.org JSON-LD
    let jsonld_stores = extract_jsonld_locations(&html);
    if !jsonld_stores.is_empty() {
        tracing::debug!(
            locator_url,
            count = jsonld_stores.len(),
            "extracted locations from JSON-LD"
        );
        return Ok(jsonld_stores);
    }

    // Strategy 4: Embedded JSON arrays in script tags
    let embed_stores = extract_json_embed_locations(&html);
    if !embed_stores.is_empty() {
        tracing::debug!(
            locator_url,
            count = embed_stores.len(),
            "extracted locations from embedded JSON"
        );
        return Ok(embed_stores);
    }

    // Strategy 5: give up gracefully
    tracing::warn!(locator_url, "no parseable locator found");
    Ok(vec![])
}

/// Compute a stable dedup key for a location.
///
/// SHA-256 over `brand_id || name || city || state || zip`, normalised to
/// lower-case city/name, upper-case state. Hex-encoded.
#[must_use]
pub fn make_location_key(brand_id: i64, loc: &RawStoreLocation) -> String {
    use sha2::{Digest, Sha256};
    let input = format!(
        "{}\x00{}\x00{}\x00{}\x00{}",
        brand_id,
        loc.name.to_lowercase().trim(),
        loc.city.as_deref().unwrap_or("").trim().to_lowercase(),
        loc.state.as_deref().unwrap_or("").trim().to_uppercase(),
        loc.zip.as_deref().unwrap_or("").trim(),
    );
    format!("{:x}", Sha256::digest(input.as_bytes()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use formats::{
        extract_balanced_array, extract_json_embed_locations, extract_jsonld_locations,
        extract_locally_company_id, extract_storemapper_token,
    };

    // -----------------------------------------------------------------------
    // make_location_key
    // -----------------------------------------------------------------------

    #[test]
    fn location_key_is_deterministic() {
        let loc = RawStoreLocation {
            external_id: None,
            name: "Whole Foods Market".to_string(),
            address_line1: Some("123 Main St".to_string()),
            city: Some("Austin".to_string()),
            state: Some("TX".to_string()),
            zip: Some("78701".to_string()),
            country: Some("US".to_string()),
            latitude: Some(30.2672),
            longitude: Some(-97.7431),
            phone: None,
            locator_source: "locally".to_string(),
            raw_data: serde_json::Value::Null,
        };

        let key1 = make_location_key(42, &loc);
        let key2 = make_location_key(42, &loc);
        assert_eq!(key1, key2, "key must be deterministic");
        assert_eq!(key1.len(), 64, "SHA-256 hex is 64 chars");
    }

    #[test]
    fn location_key_differs_for_different_inputs() {
        let base = RawStoreLocation {
            external_id: None,
            name: "Whole Foods".to_string(),
            address_line1: None,
            city: Some("Austin".to_string()),
            state: Some("TX".to_string()),
            zip: Some("78701".to_string()),
            country: None,
            latitude: None,
            longitude: None,
            phone: None,
            locator_source: "locally".to_string(),
            raw_data: serde_json::Value::Null,
        };

        let key_brand_1 = make_location_key(1, &base);
        let key_brand_2 = make_location_key(2, &base);
        assert_ne!(
            key_brand_1, key_brand_2,
            "different brand_id => different key"
        );

        let mut other_city = base.clone();
        other_city.city = Some("Dallas".to_string());
        let key_dallas = make_location_key(1, &other_city);
        assert_ne!(key_brand_1, key_dallas, "different city => different key");

        let mut other_state = base.clone();
        other_state.state = Some("CA".to_string());
        let key_ca = make_location_key(1, &other_state);
        assert_ne!(key_brand_1, key_ca, "different state => different key");
    }

    #[test]
    fn location_key_normalises_case() {
        let make = |name: &str, city: &str, state: &str| RawStoreLocation {
            external_id: None,
            name: name.to_string(),
            address_line1: None,
            city: Some(city.to_string()),
            state: Some(state.to_string()),
            zip: None,
            country: None,
            latitude: None,
            longitude: None,
            phone: None,
            locator_source: "jsonld".to_string(),
            raw_data: serde_json::Value::Null,
        };

        let lower = make("whole foods", "austin", "tx");
        let mixed = make("Whole Foods", "Austin", "TX");
        assert_eq!(
            make_location_key(1, &lower),
            make_location_key(1, &mixed),
            "name/city/state case is normalised before hashing"
        );
    }

    // -----------------------------------------------------------------------
    // Locally.com company ID extraction
    // -----------------------------------------------------------------------

    #[test]
    fn extracts_locally_company_id_from_widget_var() {
        let html = r#"
            <script>
                locallyWidgetCompanyId = 98765;
                locally.init();
            </script>
        "#;
        assert_eq!(extract_locally_company_id(html).as_deref(), Some("98765"));
    }

    #[test]
    fn extracts_locally_company_id_from_api_url() {
        let html = r#"
            <script src="https://api.locally.com/stores/json?company_id=11111&take=500"></script>
        "#;
        assert_eq!(extract_locally_company_id(html).as_deref(), Some("11111"));
    }

    #[test]
    fn extracts_locally_company_id_from_plain_param() {
        // A page that references locally.com AND uses the bare company_id param.
        let html = r#"<script src="https://widget.locally.com/locally.js"></script><div data-company_id=55555></div>"#;
        // The locally.com signal fires the pre-filter; the company_id pattern extracts the ID.
        assert_eq!(extract_locally_company_id(html).as_deref(), Some("55555"));
    }

    #[test]
    fn returns_none_for_plain_company_id_without_locally_signal() {
        // bare company_id with no locally.com / locallyWidgetCompanyId => must not match
        let html = r#"<div data-crm-company_id="99999"></div>"#;
        assert_eq!(
            extract_locally_company_id(html),
            None,
            "bare company_id without a Locally signal must not match"
        );
    }

    #[test]
    fn returns_none_when_no_locally_signals() {
        let html = r#"<html><body><p>No store locator here.</p></body></html>"#;
        assert_eq!(extract_locally_company_id(html), None);
    }

    // -----------------------------------------------------------------------
    // JSON-LD extraction
    // -----------------------------------------------------------------------

    #[test]
    fn extracts_local_business_from_jsonld() {
        let html = r#"
            <html><head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "Green Leaf Dispensary",
                "address": {
                    "@type": "PostalAddress",
                    "streetAddress": "456 Elm Ave",
                    "addressLocality": "Portland",
                    "addressRegion": "OR",
                    "postalCode": "97201",
                    "addressCountry": "US"
                },
                "geo": {
                    "@type": "GeoCoordinates",
                    "latitude": "45.5051",
                    "longitude": "-122.6750"
                },
                "telephone": "+1-503-555-0100"
            }
            </script>
            </head></html>
        "#;

        let locs = extract_jsonld_locations(html);
        assert_eq!(locs.len(), 1);
        let loc = &locs[0];
        assert_eq!(loc.name, "Green Leaf Dispensary");
        assert_eq!(loc.city.as_deref(), Some("Portland"));
        assert_eq!(loc.state.as_deref(), Some("OR"));
        assert_eq!(loc.zip.as_deref(), Some("97201"));
        assert_eq!(loc.country.as_deref(), Some("US"));
        assert_eq!(loc.address_line1.as_deref(), Some("456 Elm Ave"));
        assert!((loc.latitude.unwrap() - 45.5051_f64).abs() < 1e-4);
        assert!((loc.longitude.unwrap() - (-122.6750_f64)).abs() < 1e-4);
        assert_eq!(loc.phone.as_deref(), Some("+1-503-555-0100"));
        assert_eq!(loc.locator_source, "jsonld");
    }

    #[test]
    fn skips_jsonld_non_location_types() {
        let html = r#"
            <html><head>
            <script type="application/ld+json">
            {"@type": "Article", "name": "How to buy hemp beverages"}
            </script>
            </head></html>
        "#;
        let locs = extract_jsonld_locations(html);
        assert!(
            locs.is_empty(),
            "Article type should not produce a location"
        );
    }

    #[test]
    fn extracts_multiple_locations_from_jsonld_array() {
        let html = r#"
            <html><head>
            <script type="application/ld+json">
            [
                {"@type": "Store", "name": "Store A", "address": {"addressLocality": "Seattle", "addressRegion": "WA"}},
                {"@type": "Store", "name": "Store B", "address": {"addressLocality": "Tacoma",  "addressRegion": "WA"}}
            ]
            </script>
            </head></html>
        "#;
        let locs = extract_jsonld_locations(html);
        assert_eq!(locs.len(), 2);
        assert_eq!(locs[0].name, "Store A");
        assert_eq!(locs[1].name, "Store B");
    }

    #[test]
    fn jsonld_type_as_array_is_accepted() {
        // `@type` may be an array in the wild; any matching element should pass.
        let html = r#"
            <html><head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": ["LocalBusiness", "GroceryStore"],
                "name": "Corner Market",
                "address": {
                    "addressLocality": "Denver",
                    "addressRegion": "CO"
                }
            }
            </script>
            </head></html>
        "#;
        let locs = extract_jsonld_locations(html);
        assert_eq!(
            locs.len(),
            1,
            "array @type containing LocalBusiness should match"
        );
        assert_eq!(locs[0].name, "Corner Market");
        assert_eq!(locs[0].city.as_deref(), Some("Denver"));
    }

    // -----------------------------------------------------------------------
    // Storemapper token extraction
    // -----------------------------------------------------------------------

    #[test]
    fn extracts_storemapper_token_from_data_attribute() {
        let html = r#"
            <div id="storemapper"
                 data-storemapper-token="abc123"
                 class="store-locator">
            </div>
        "#;
        assert_eq!(
            extract_storemapper_token(html).as_deref(),
            Some("abc123"),
            "should extract token from data-storemapper-token attribute"
        );
    }

    #[test]
    fn extracts_storemapper_token_from_api_url() {
        let html = r#"
            <script>
                var smUrl = "https://storemapper.co/api/stores?token=xyz789";
            </script>
        "#;
        assert_eq!(
            extract_storemapper_token(html).as_deref(),
            Some("xyz789"),
            "should extract token from storemapper API URL"
        );
    }

    #[test]
    fn returns_none_when_no_storemapper_signal() {
        let html = r#"<html><body><p>No store locator here.</p></body></html>"#;
        assert_eq!(extract_storemapper_token(html), None);
    }

    // -----------------------------------------------------------------------
    // Embedded JSON array extraction
    // -----------------------------------------------------------------------

    #[test]
    fn extracts_stores_from_embedded_json_script_block() {
        let html = r#"
            <html><body>
            <script>
            var stores = [
                {"name": "Hemp House", "city": "Austin", "state": "TX", "lat": 30.26, "lng": -97.74},
                {"name": "CBD Depot",  "city": "Dallas", "state": "TX", "lat": 32.77, "lng": -96.79}
            ];
            </script>
            </body></html>
        "#;
        let locs = extract_json_embed_locations(html);
        assert_eq!(locs.len(), 2, "should extract both store objects");
        assert_eq!(locs[0].name, "Hemp House");
        assert_eq!(locs[0].city.as_deref(), Some("Austin"));
        assert_eq!(locs[1].name, "CBD Depot");
        assert_eq!(locs[1].city.as_deref(), Some("Dallas"));
        assert!(locs.iter().all(|l| l.locator_source == "json_embed"));
    }

    #[test]
    fn extract_balanced_array_rejects_mismatched_closer() {
        // `[42}` should NOT be accepted — depth hits 0 on `}` which is not `]`.
        // The function must continue scanning and ultimately return None.
        assert_eq!(
            extract_balanced_array("[42}"),
            None,
            "mismatched closing brace must not be returned as a valid array"
        );
    }

    #[test]
    fn extract_balanced_array_accepts_nested_objects() {
        let s = r#"[{"a": 1}, {"b": 2}] trailing"#;
        let result = extract_balanced_array(s);
        assert_eq!(result, Some(r#"[{"a": 1}, {"b": 2}]"#));
    }
}
