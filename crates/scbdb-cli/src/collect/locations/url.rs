//! Locator URL resolution: configured URL or HTTP auto-discovery.

/// Common store-locator path candidates tried during auto-discovery.
pub(super) const LOCATOR_PATHS: &[&str] = &[
    "/pages/where-to-buy",
    "/pages/store-locator",
    "/pages/storelocator",
    "/pages/find-us",
    "/pages/locations",
    "/pages/retailers",
    "/pages/find",
    "/pages/beverage-finder",
    "/locator",
    "/storelocator",
    "/find-products",
    "/find",
    "/beverage-finder",
    "/stores",
];

/// Resolve a locator URL for a brand.
///
/// Returns `brand.store_locator_url` if set, otherwise delegates to
/// [`discover_locator_url`].
pub(super) async fn resolve_locator_url(
    brand: &scbdb_db::BrandRow,
    config: &scbdb_core::AppConfig,
) -> Option<String> {
    if let Some(url) = &brand.store_locator_url {
        return Some(url.clone());
    }

    discover_locator_url(
        brand,
        config.scraper_request_timeout_secs,
        &config.scraper_user_agent,
    )
    .await
}

/// Probe common store-locator paths on a brand's domain and return the first
/// URL that responds with a 2xx status.
///
/// Uses a 5-second HEAD timeout regardless of the main scraper timeout,
/// since this is just URL probing.
async fn discover_locator_url(
    brand: &scbdb_db::BrandRow,
    _timeout_secs: u64,
    user_agent: &str,
) -> Option<String> {
    let domain = brand.domain.as_deref()?;

    // Normalise the domain to a base URL with scheme.
    let base_url = if domain.starts_with("http://") || domain.starts_with("https://") {
        domain.trim_end_matches('/').to_string()
    } else {
        format!("https://{}", domain.trim_end_matches('/'))
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;

    for path in LOCATOR_PATHS {
        let url = format!("{base_url}{path}");
        let result = client
            .head(&url)
            .header(reqwest::header::USER_AGENT, user_agent)
            .send()
            .await;

        if let Ok(resp) = result {
            if resp.status().is_success() {
                tracing::debug!(brand = %brand.slug, url = %url, "discovered locator URL");
                return Some(url);
            }
        }
    }

    None
}
