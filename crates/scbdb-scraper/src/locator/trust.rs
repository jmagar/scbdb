//! Trust validation and deduplication key utilities for store locator results.

use super::types::RawStoreLocation;

/// Validate whether a scrape result is trusted enough to mutate stored data.
///
/// Trusted sources: `locally`, `storemapper`, `stockist`, `storepoint`,
/// `roseperl`, `vtinfo`, `askhoodie`, `beveragefinder`, `storerocket`,
/// `agile_store_locator`, `jsonld`.
///
/// `json_embed` is a fallback parser and is accepted only when quality is
/// high enough to reduce false positives.
///
/// # Errors
///
/// Returns `Err` with a human-readable reason when the scrape result is not
/// trusted (empty result, low-quality embed parse, or unknown source).
pub fn validate_store_locations_trust(locations: &[RawStoreLocation]) -> Result<(), String> {
    if locations.is_empty() {
        return Err("scrape returned zero locations".to_string());
    }

    let source = locations
        .first()
        .map_or("unknown", |loc| loc.locator_source.as_str());

    match source {
        "locally"
        | "storemapper"
        | "stockist"
        | "storepoint"
        | "roseperl"
        | "vtinfo"
        | "askhoodie"
        | "beveragefinder"
        | "storerocket"
        | "agile_store_locator"
        | "jsonld"
        | "destini" => Ok(()),
        "json_embed" => {
            let quality_count = locations
                .iter()
                .filter(|loc| location_record_has_minimum_shape(loc))
                .count();
            // Both counts are bounded by the slice length which is at most
            // usize::MAX; for any realistic data set they fit well within
            // f64's 52-bit mantissa without precision loss.
            #[allow(clippy::cast_precision_loss)]
            let quality_ratio = quality_count as f64 / locations.len() as f64;

            if locations.len() >= 5 && quality_ratio >= 0.80 {
                Ok(())
            } else {
                Err(format!(
                    "json_embed scrape below trust threshold (count={}, quality_ratio={quality_ratio:.2})",
                    locations.len()
                ))
            }
        }
        other => Err(format!("unknown locator source '{other}'")),
    }
}

fn location_record_has_minimum_shape(location: &RawStoreLocation) -> bool {
    let has_name = !location.name.trim().is_empty();
    let has_address = location
        .address_line1
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty());
    let has_city_state = location
        .city
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty())
        && location
            .state
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty());
    let has_coordinates = location.latitude.is_some() && location.longitude.is_some();

    has_name && (has_address || has_city_state || has_coordinates)
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
