//! Strategy 4: Embedded JSON array extraction from script tags.

use regex::Regex;

use crate::locator::types::RawStoreLocation;

/// Scan `<script>` tag contents for JSON arrays whose objects look like
/// store records (have a name-like field AND an address/city/lat field).
pub(in crate::locator) fn extract_json_embed_locations(html: &str) -> Vec<RawStoreLocation> {
    let script_re = Regex::new(r"(?is)<script\b[^>]*>(.*?)</script>").expect("valid regex");

    // Quick pre-filter: the array must contain objects with both a name-like
    // field and an address/location field.
    let candidate_re = Regex::new(
        r#"(?is)\[\s*\{[^}]*"(?:name|store_name|Name)"[^}]*"(?:city|lat|address|latitude)"[^}]*\}"#,
    )
    .expect("valid regex");

    for cap in script_re.captures_iter(html) {
        let content = match cap.get(1) {
            Some(m) => m.as_str(),
            None => continue,
        };

        if content.is_empty() {
            continue;
        }

        // Find candidate array positions in the script content.
        for m in candidate_re.find_iter(content) {
            // The regex starts with `\[`, so m.start() IS the opening `[`.
            let start = m.start();

            // Try to extract a balanced JSON array starting at `start`.
            if let Some(array_str) = extract_balanced_array(&content[start..]) {
                if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str(array_str) {
                    let locations: Vec<RawStoreLocation> = arr
                        .into_iter()
                        .filter_map(|obj| embed_object_to_location(&obj))
                        .collect();
                    if !locations.is_empty() {
                        return locations;
                    }
                }
            }
        }
    }

    vec![]
}

/// Try to extract a balanced JSON array from the start of `s`.
///
/// Scans `s` character-by-character tracking bracket depth, respecting
/// string literals and escape sequences. Returns the shortest prefix of `s`
/// that forms a complete `[…]` array, or `None` if the array is unterminated.
/// Only `]` (not `}`) at depth 0 triggers a return, so malformed input like
/// `[42}` is never accepted.
pub(in crate::locator) fn extract_balanced_array(s: &str) -> Option<&str> {
    if !s.starts_with('[') {
        return None;
    }
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if in_string {
            match c {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '[' | '{' => depth += 1,
            '}' => depth -= 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Convert an embedded JSON object to a `RawStoreLocation` using common field
/// name patterns found in store data widgets.
fn embed_object_to_location(obj: &serde_json::Value) -> Option<RawStoreLocation> {
    if !obj.is_object() {
        return None;
    }

    // Must have a name-like field.
    let name = obj
        .get("name")
        .or_else(|| obj.get("store_name"))
        .or_else(|| obj.get("Name"))
        .and_then(|v| v.as_str())?
        .to_string();

    let address_line1 = obj
        .get("address")
        .or_else(|| obj.get("address1"))
        .or_else(|| obj.get("street"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let city = obj
        .get("city")
        .or_else(|| obj.get("City"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let state = obj
        .get("state")
        .or_else(|| obj.get("State"))
        .or_else(|| obj.get("province"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let zip = obj
        .get("zip")
        .or_else(|| obj.get("postal_code"))
        .or_else(|| obj.get("postcode"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let country = obj
        .get("country")
        .or_else(|| obj.get("Country"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let latitude = obj
        .get("lat")
        .or_else(|| obj.get("latitude"))
        .or_else(|| obj.get("Lat"))
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        });

    let longitude = obj
        .get("lng")
        .or_else(|| obj.get("longitude"))
        .or_else(|| obj.get("Lng"))
        .or_else(|| obj.get("lon"))
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        });

    let phone = obj
        .get("phone")
        .or_else(|| obj.get("Phone"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let external_id = obj.get("id").and_then(|v| {
        v.as_str()
            .map(str::to_string)
            .or_else(|| Some(v.to_string()))
    });

    // Require at least a city or lat to avoid matching non-store objects.
    if city.is_none() && latitude.is_none() && address_line1.is_none() {
        return None;
    }

    Some(RawStoreLocation {
        external_id,
        name,
        address_line1,
        city,
        state,
        zip,
        country,
        latitude,
        longitude,
        phone,
        locator_source: "json_embed".to_string(),
        raw_data: obj.clone(),
    })
}
