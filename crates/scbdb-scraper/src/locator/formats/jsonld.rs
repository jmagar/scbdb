//! Strategy 3: schema.org JSON-LD extraction.

use regex::Regex;

use crate::locator::types::RawStoreLocation;

/// Extract store locations from `<script type="application/ld+json">` blocks.
pub(in crate::locator) fn extract_jsonld_locations(html: &str) -> Vec<RawStoreLocation> {
    let script_re = Regex::new(
        r#"(?is)<script[^>]+type\s*=\s*["']application/ld\+json["'][^>]*>(.*?)</script>"#,
    )
    .expect("valid regex");

    let mut results = Vec::new();

    for cap in script_re.captures_iter(html) {
        let json_text = match cap.get(1) {
            Some(m) => m.as_str(),
            None => continue,
        };

        let value: serde_json::Value = match serde_json::from_str(json_text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Accept top-level object, array, or @graph container.
        let mut candidates: Vec<serde_json::Value> = if value.is_array() {
            value
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect()
        } else {
            vec![value]
        };

        // Expand @graph containers: many sites wrap structured data inside
        // {"@graph": [...]} at the top level.
        let mut expanded = Vec::new();
        for item in &candidates {
            if let Some(graph) = item.get("@graph").and_then(serde_json::Value::as_array) {
                expanded.extend(graph.iter().cloned());
            }
        }
        candidates.extend(expanded);

        for item in candidates {
            if let Some(loc) = jsonld_item_to_location(&item) {
                results.push(loc);
            }
        }
    }

    results
}

/// Convert a single JSON-LD object to a `RawStoreLocation`, if it represents
/// a physical location (`LocalBusiness`, `Store`, or `FoodEstablishment`).
fn jsonld_item_to_location(item: &serde_json::Value) -> Option<RawStoreLocation> {
    let type_node = item.get("@type")?;
    let accepted_types = [
        "LocalBusiness",
        "Store",
        "FoodEstablishment",
        "GroceryStore",
        "ConvenienceStore",
        "DrinkingEstablishment",
        "BarOrPub",
        "Brewery",
    ];

    // `@type` may be a plain string OR an array of strings (e.g.
    // `["LocalBusiness", "GroceryStore"]`). Accept the item if any element
    // matches one of the accepted types.
    let type_matches = if let Some(s) = type_node.as_str() {
        accepted_types.iter().any(|t| s.eq_ignore_ascii_case(t))
    } else if let Some(arr) = type_node.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str())
            .any(|s| accepted_types.iter().any(|t| s.eq_ignore_ascii_case(t)))
    } else {
        false
    };
    if !type_matches {
        return None;
    }

    let name = item.get("name")?.as_str()?.to_string();
    let address = item.get("address");
    let geo = item.get("geo");

    let address_line1 = address
        .and_then(|a| a.get("streetAddress"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let city = address
        .and_then(|a| a.get("addressLocality"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let state = address
        .and_then(|a| a.get("addressRegion"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let zip = address
        .and_then(|a| a.get("postalCode"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let country = address
        .and_then(|a| a.get("addressCountry"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    // geo.latitude / geo.longitude may be strings or numbers in the wild.
    let latitude = geo.and_then(|g| g.get("latitude")).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
    });
    let longitude = geo.and_then(|g| g.get("longitude")).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
    });

    let phone = item
        .get("telephone")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    Some(RawStoreLocation {
        external_id: None,
        name,
        address_line1,
        city,
        state,
        zip,
        country,
        latitude,
        longitude,
        phone,
        locator_source: "jsonld".to_string(),
        raw_data: item.clone(),
    })
}
