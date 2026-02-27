//! Per-brand collection helpers.
//!
//! These functions are called only from the top-level collect runners.
//! They handle the Shopify fetch → normalize → persist pipeline for a single
//! brand and record per-brand status rows.

mod pipeline;

use pipeline::collect_brand_core;

pub(super) fn build_shopify_client(
    config: &scbdb_core::AppConfig,
) -> anyhow::Result<scbdb_scraper::ShopifyClient> {
    scbdb_scraper::ShopifyClient::new(
        config.scraper_request_timeout_secs,
        &config.scraper_user_agent,
        config.scraper_max_retries,
        config.scraper_retry_backoff_base_secs,
    )
    .map_err(|e| anyhow::anyhow!("failed to build Shopify client: {e}"))
}

/// Upserts products, variants, and price snapshots for a pre-normalized product
/// list.
///
/// Returns `(products_count, snapshots_count)` on success. Propagates DB
/// errors to the caller so per-brand failure handling can be applied.
pub(super) async fn persist_normalized_products(
    pool: &sqlx::PgPool,
    brand_id: i64,
    run_id: i64,
    normalized_products: &[scbdb_core::NormalizedProduct],
) -> anyhow::Result<(i32, i32)> {
    let mut brand_products: i32 = 0;
    let mut brand_snapshots: i32 = 0;

    for normalized in normalized_products {
        let product_id = scbdb_db::upsert_product(pool, brand_id, normalized).await?;
        for variant in &normalized.variants {
            let variant_id = scbdb_db::upsert_variant(pool, product_id, variant).await?;
            let inserted = scbdb_db::insert_price_snapshot_if_changed(
                pool,
                variant_id,
                Some(run_id),
                &variant.price,
                variant.compare_at_price.as_deref(),
                &variant.currency_code,
                variant.source_url.as_deref(),
            )
            .await?;
            if inserted {
                brand_snapshots = brand_snapshots.saturating_add(1);
            }
        }
        brand_products = brand_products.saturating_add(1);
    }

    Ok((brand_products, brand_snapshots))
}

/// Returns `(records_count, brand_succeeded)`.
///
/// When `brand_succeeded` is `false`, the brand's failure has already been
/// recorded in `collection_run_brands` by [`collect_brand_core`] and the
/// returned count is `0`.
pub(super) async fn collect_brand_products(
    pool: &sqlx::PgPool,
    client: &scbdb_scraper::ShopifyClient,
    config: &scbdb_core::AppConfig,
    run_id: i64,
    brand: &scbdb_db::BrandRow,
) -> anyhow::Result<(i32, bool)> {
    match collect_brand_core(pool, client, config, run_id, brand).await {
        Ok((brand_products, _brand_snapshots, partial_note)) => {
            let status = if partial_note.is_some() {
                "partial"
            } else {
                "succeeded"
            };
            // The `partial_note` (e.g. "browser-profile fallback succeeded") is
            // stored in the `error_message` column of `collection_run_brands` because
            // the table has no dedicated `note` column. We prefix with "[NOTE]" to
            // distinguish informational notes from actual errors.
            let prefixed_note = partial_note.map(|note| format!("[NOTE] {note}"));
            if let Err(e) = scbdb_db::upsert_collection_run_brand(
                pool,
                run_id,
                brand.id,
                status,
                Some(brand_products),
                prefixed_note.as_deref(),
            )
            .await
            {
                tracing::error!(
                    brand = %brand.slug,
                    run_id,
                    error = %e,
                    "product data saved but failed to record brand success in collection_run_brands — audit trail incomplete"
                );
                return Err(e.into());
            }
            Ok((brand_products, true))
        }
        Err(e) => {
            // collect_brand_core already recorded the failure row and logged it.
            tracing::error!(brand = %brand.slug, error = %e, "brand collection failed");
            Ok((0, false))
        }
    }
}

/// Collect brand pricing snapshot data.
///
/// Fetches the current Shopify catalog, upserts any new products/variants
/// encountered, then captures price snapshots for all variants.
///
/// Returns `(products_count, snapshots_count, brand_succeeded)`.
///
/// When `brand_succeeded` is `false`, the brand's failure has already been
/// recorded in `collection_run_brands` by [`collect_brand_core`] and both
/// returned counts are `0`.
pub(super) async fn collect_brand_pricing(
    pool: &sqlx::PgPool,
    client: &scbdb_scraper::ShopifyClient,
    config: &scbdb_core::AppConfig,
    run_id: i64,
    brand: &scbdb_db::BrandRow,
) -> anyhow::Result<(i32, i32, bool)> {
    match collect_brand_core(pool, client, config, run_id, brand).await {
        Ok((brand_products, brand_snapshots, partial_note)) => {
            let status = if partial_note.is_some() {
                "partial"
            } else {
                "succeeded"
            };
            // See comment in `collect_brand_products` about [NOTE] prefix.
            let prefixed_note = partial_note.map(|note| format!("[NOTE] {note}"));
            if let Err(e) = scbdb_db::upsert_collection_run_brand(
                pool,
                run_id,
                brand.id,
                status,
                Some(brand_snapshots),
                prefixed_note.as_deref(),
            )
            .await
            {
                tracing::error!(
                    brand = %brand.slug,
                    run_id,
                    error = %e,
                    "pricing data saved but failed to record brand success in collection_run_brands — audit trail incomplete"
                );
                return Err(e.into());
            }
            Ok((brand_products, brand_snapshots, true))
        }
        Err(e) => {
            // collect_brand_core already recorded the failure row and logged it.
            tracing::error!(brand = %brand.slug, error = %e, "brand collection failed");
            Ok((0, 0, false))
        }
    }
}

#[cfg(test)]
#[path = "../brand_test.rs"]
mod tests;
