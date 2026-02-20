//! Per-brand collection helpers.
//!
//! These functions are called only from the top-level collect runners
//! in [`super`]. They handle the Shopify fetch → normalize → persist
//! pipeline for a single brand and record per-brand status rows.

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

/// Shared core for both product and pricing collection runs.
///
/// Fetches the Shopify catalog for a single brand, normalizes all products,
/// and upserts products, variants, and price snapshots to the database.
///
/// Returns `Ok((products_count, snapshots_count))` on success. The caller is
/// responsible for recording success via `upsert_collection_run_brand` with
/// whichever count it wants to surface (products or snapshots).
///
/// Returns `Err` when a network fetch or DB persist failure occurs. Before
/// returning the error, this function records a `"failed"` status row in
/// `collection_run_brands` on a best-effort basis — the caller must not write
/// a second status row for the brand.
pub(super) async fn collect_brand_core(
    pool: &sqlx::PgPool,
    client: &scbdb_scraper::ShopifyClient,
    config: &scbdb_core::AppConfig,
    run_id: i64,
    brand: &scbdb_db::BrandRow,
) -> anyhow::Result<(i32, i32)> {
    let shop_url = brand.shop_url.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "brand '{}' has no shop_url — the filter in load_brands_for_collect should have excluded it; this is a bug",
            brand.slug
        )
    })?;

    let raw_products = match client
        .fetch_all_products(shop_url, 250, config.scraper_inter_request_delay_ms)
        .await
    {
        Ok(products) => products,
        Err(e) => {
            let err_string = e.to_string();
            tracing::error!(
                brand = %brand.slug,
                error = %err_string,
                "failed to fetch products for brand"
            );
            if let Err(mark_err) = scbdb_db::upsert_collection_run_brand(
                pool,
                run_id,
                brand.id,
                "failed",
                None,
                Some(&err_string),
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
            return Err(anyhow::anyhow!(
                "failed to fetch products for {}: {}",
                brand.slug,
                err_string
            ));
        }
    };

    // Normalize all products first, then persist in a single block so DB
    // errors are captured per-brand rather than propagated.
    let normalized_products: Vec<_> = raw_products
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

    match persist_normalized_products(pool, brand.id, run_id, &normalized_products).await {
        Ok(counts) => Ok(counts),
        Err(e) => {
            let err_string = format!("{e:#}");
            tracing::error!(
                brand = %brand.slug,
                error = %err_string,
                "db error persisting brand products"
            );
            if let Err(mark_err) = scbdb_db::upsert_collection_run_brand(
                pool,
                run_id,
                brand.id,
                "failed",
                None,
                Some(&err_string),
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
            Err(anyhow::anyhow!(
                "db error persisting products for {}: {}",
                brand.slug,
                err_string
            ))
        }
    }
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
        Ok((brand_products, _brand_snapshots)) => {
            if let Err(e) = scbdb_db::upsert_collection_run_brand(
                pool,
                run_id,
                brand.id,
                "succeeded",
                Some(brand_products),
                None,
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
/// encountered, then captures price snapshots for all variants. New products
/// are persisted as a side effect so that pricing data is never lost when a
/// brand adds products between collection runs.
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
        Ok((brand_products, brand_snapshots)) => {
            if let Err(e) = scbdb_db::upsert_collection_run_brand(
                pool,
                run_id,
                brand.id,
                "succeeded",
                Some(brand_snapshots),
                None,
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Insert a minimal brand row for test purposes and return its generated `id`.
    async fn insert_test_brand(pool: &sqlx::PgPool, slug: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "INSERT INTO brands (name, slug, relationship, tier, shop_url, is_active) \
             VALUES ($1, $2, 'portfolio', 1, $3, true) RETURNING id",
        )
        .bind(format!("Test Brand {slug}"))
        .bind(slug)
        .bind(format!("https://{slug}.com"))
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("insert_test_brand failed for slug '{slug}': {e}"))
    }

    fn make_normalized_product(source_product_id: &str) -> scbdb_core::NormalizedProduct {
        scbdb_core::NormalizedProduct {
            source_product_id: source_product_id.to_string(),
            source_platform: "shopify".to_string(),
            name: "Test Product".to_string(),
            description: None,
            product_type: None,
            tags: vec![],
            handle: Some("test-product".to_string()),
            status: "active".to_string(),
            source_url: None,
            variants: vec![make_normalized_variant("VAR-001")],
        }
    }

    fn make_normalized_variant(source_variant_id: &str) -> scbdb_core::NormalizedVariant {
        scbdb_core::NormalizedVariant {
            source_variant_id: source_variant_id.to_string(),
            sku: None,
            title: "Default Title".to_string(),
            price: "12.99".to_string(),
            compare_at_price: None,
            currency_code: "USD".to_string(),
            source_url: None,
            is_available: true,
            is_default: true,
            dosage_mg: Some(5.0),
            cbd_mg: None,
            size_value: Some(12.0),
            size_unit: Some("oz".to_string()),
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn persist_normalized_products_deduplicates_snapshots(pool: sqlx::PgPool) {
        let brand_id = insert_test_brand(&pool, "dedup-brand").await;

        let run = scbdb_db::create_collection_run(&pool, "products", "cli")
            .await
            .expect("create_collection_run failed");
        scbdb_db::start_collection_run(&pool, run.id)
            .await
            .expect("start_collection_run failed");

        let product = make_normalized_product("DEDUP-PROD-001");

        // First call: should insert 1 product and 1 snapshot.
        let (products, snapshots) =
            persist_normalized_products(&pool, brand_id, run.id, std::slice::from_ref(&product))
                .await
                .expect("first persist_normalized_products failed");
        assert_eq!(
            (products, snapshots),
            (1, 1),
            "first call should insert 1 product and 1 snapshot"
        );

        // Second call with the same price: should process 1 product but insert 0 new snapshots.
        let (products, snapshots) =
            persist_normalized_products(&pool, brand_id, run.id, std::slice::from_ref(&product))
                .await
                .expect("second persist_normalized_products failed");
        assert_eq!(
            (products, snapshots),
            (1, 0),
            "second call with same price should insert 0 new snapshots (dedup)"
        );
    }
}
