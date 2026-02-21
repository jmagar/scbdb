//! Core per-brand collection orchestration.
//!
//! `collect_brand_core` handles the full Shopify fetch → logo → normalize →
//! filter → persist pipeline for a single brand, including error recovery
//! and 403-fallback logic.

/// Brand slugs known to return HTTP 403 on the standard scraper user-agent.
/// These receive a browser-profile retry rather than an immediate failure.
///
/// NOTE: This list is hardcoded and requires a code change for new brands.
/// Ideally, this should be driven by a flag in `config/brands.yaml` (e.g.
/// `requires_browser_profile: true`) so new 403 brands can be added without
/// recompilation.
pub(super) const KNOWN_403_FALLBACK_BRANDS: &[&str] = &["cycling-frog"];

/// Shared core for both product and pricing collection runs.
///
/// Fetches the Shopify catalog for a single brand, normalizes all products,
/// and upserts products, variants, and price snapshots to the database.
///
/// Returns `Ok((products_count, snapshots_count, partial_note))` on success.
/// The caller is responsible for recording success via
/// `upsert_collection_run_brand` with whichever count it wants to surface.
///
/// Returns `Err` when a network fetch or DB persist failure occurs. Before
/// returning the error, this function records a `"failed"` status row in
/// `collection_run_brands` on a best-effort basis — the caller must not write
/// a second status row for the brand.
#[allow(clippy::too_many_lines)] // Orchestration function: product fetch, normalization, upsert loop, error handling
pub(super) async fn collect_brand_core(
    pool: &sqlx::PgPool,
    client: &scbdb_scraper::ShopifyClient,
    config: &scbdb_core::AppConfig,
    run_id: i64,
    brand: &scbdb_db::BrandRow,
) -> anyhow::Result<(i32, i32, Option<String>)> {
    let shop_url = brand.shop_url.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "brand '{}' has no shop_url — the filter in load_brands_for_collect should have excluded it; this is a bug",
            brand.slug
        )
    })?;

    match scbdb_scraper::fetch_brand_logo_url(
        shop_url,
        config.scraper_request_timeout_secs,
        &config.scraper_user_agent,
    )
    .await
    {
        Ok(Some(logo_url)) => {
            if let Err(e) = scbdb_db::update_brand_logo(pool, brand.id, &logo_url).await {
                tracing::warn!(
                    brand = %brand.slug,
                    error = %e,
                    "failed to persist brand logo URL"
                );
            }
        }
        Ok(None) => {
            tracing::debug!(brand = %brand.slug, "no logo candidate found");
        }
        Err(e) => {
            tracing::warn!(
                brand = %brand.slug,
                error = %e,
                "logo extraction failed; continuing collection"
            );
        }
    }

    let mut partial_note: Option<String> = None;
    let raw_products = match client
        .fetch_all_products(shop_url, 250, config.scraper_inter_request_delay_ms)
        .await
    {
        Ok(products) => products,
        Err(e) => {
            let primary_err = e.to_string();
            let is_known_403 = matches!(
                e,
                scbdb_scraper::ScraperError::UnexpectedStatus { status: 403, .. }
            ) && KNOWN_403_FALLBACK_BRANDS.contains(&brand.slug.as_str());

            if is_known_403 {
                tracing::warn!(
                    brand = %brand.slug,
                    error = %primary_err,
                    "known 403 storefront; retrying with browser-profile headers"
                );

                match client
                    .fetch_all_products_browser_profile(
                        shop_url,
                        250,
                        config.scraper_inter_request_delay_ms,
                    )
                    .await
                {
                    Ok(products) => {
                        partial_note = Some(
                            "primary products.json fetch returned 403; browser-profile fallback succeeded"
                                .to_string(),
                        );
                        products
                    }
                    Err(fallback_err) => {
                        let err_string = format!(
                            "{primary_err}; browser-profile fallback failed: {fallback_err}"
                        );
                        tracing::error!(
                            brand = %brand.slug,
                            error = %err_string,
                            "failed to fetch products for brand"
                        );
                        record_failure(pool, run_id, brand, &err_string).await;
                        return Err(anyhow::anyhow!(
                            "failed to fetch products for {}: {}",
                            brand.slug,
                            err_string
                        ));
                    }
                }
            } else {
                let err_string = primary_err;
                tracing::error!(
                    brand = %brand.slug,
                    error = %err_string,
                    "failed to fetch products for brand"
                );
                record_failure(pool, run_id, brand, &err_string).await;
                return Err(anyhow::anyhow!(
                    "failed to fetch products for {}: {}",
                    brand.slug,
                    err_string
                ));
            }
        }
    };

    // Normalize all products first, then persist in a single block so DB
    // errors are captured per-brand rather than propagated.
    let normalized_all: Vec<_> = raw_products
        .into_iter()
        .filter_map(
            |raw_product| match scbdb_scraper::normalize_product(raw_product, shop_url) {
                Ok(p) => Some(p),
                Err(e) => {
                    tracing::warn!(
                        brand = %brand.slug,
                        error = %e,
                        "skipping product — normalization failed"
                    );
                    None
                }
            },
        )
        .collect();

    // Filter to beverage products only: keep products where at least one
    // variant has a dosage (mg) or size (oz/ml) value. This excludes merch,
    // accessories, gift cards, insurance, and other non-beverage items that
    // Shopify stores publish alongside their drink catalogs.
    let normalized_products: Vec<_> = normalized_all
        .into_iter()
        .filter(|p| {
            let keep = p
                .variants
                .iter()
                .any(|v| v.dosage_mg.is_some() || v.size_value.is_some());
            if !keep {
                tracing::debug!(
                    brand = %brand.slug,
                    product_id = %p.source_product_id,
                    name = %p.name,
                    "dropping non-beverage product — no dosage or size on any variant"
                );
            }
            keep
        })
        .collect();

    match super::persist_normalized_products(pool, brand.id, run_id, &normalized_products).await {
        Ok((products_count, snapshots_count)) => {
            Ok((products_count, snapshots_count, partial_note))
        }
        Err(e) => {
            let err_string = format!("{e:#}");
            tracing::error!(
                brand = %brand.slug,
                error = %err_string,
                "db error persisting brand products"
            );
            record_failure(pool, run_id, brand, &err_string).await;
            Err(anyhow::anyhow!(
                "db error persisting products for {}: {}",
                brand.slug,
                err_string
            ))
        }
    }
}

/// Record a `"failed"` status in `collection_run_brands` on a best-effort basis.
async fn record_failure(
    pool: &sqlx::PgPool,
    run_id: i64,
    brand: &scbdb_db::BrandRow,
    err_string: &str,
) {
    if let Err(mark_err) = scbdb_db::upsert_collection_run_brand(
        pool,
        run_id,
        brand.id,
        "failed",
        None,
        Some(err_string),
    )
    .await
    {
        tracing::error!(
            run_id,
            brand = %brand.slug,
            error = %mark_err,
            "failed to record brand failure"
        );
    }
}
