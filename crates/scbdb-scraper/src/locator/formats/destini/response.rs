//! `Destini` API response parsers.
//!
//! Converts raw Knox/productCategories JSON into domain types.

use crate::locator::types::RawStoreLocation;

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
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
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
        .filter(|v| !v.is_empty())
        .map(str::to_string)?;

    Some(RawStoreLocation {
        external_id: store.get("id").and_then(value_as_string),
        name,
        address_line1: store
            .get("address")
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
            .get("postalCode")
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
        latitude: store.get("latitude").and_then(value_as_f64),
        longitude: store.get("longitude").and_then(value_as_f64),
        phone: store
            .get("phone")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        locator_source: "destini".to_string(),
        raw_data: store.clone(),
    })
}

pub(super) fn value_as_string(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(str::to_string).or_else(|| {
        if value.is_number() {
            Some(value.to_string())
        } else {
            None
        }
    })
}

pub(super) fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
}
