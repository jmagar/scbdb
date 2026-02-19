//! Collection command handlers for the CLI.
//!
//! These are called from `main` after the database pool and config are
//! established. Per-brand failures are logged and skipped rather than
//! propagated so a single bad brand does not abort the full run.

/// Load the brands to process for a collect run.
///
/// If `brand_filter` is `Some(slug)`, fetches that single brand and returns an
/// error if not found. If `None`, returns all active brands. Either way,
/// brands without a `shop_url` are filtered out with a warning logged.
pub(crate) async fn load_brands_for_collect(
    pool: &sqlx::PgPool,
    brand_filter: Option<&str>,
) -> anyhow::Result<Vec<scbdb_db::BrandRow>> {
    let all = match brand_filter {
        Some(slug) => {
            let brand = scbdb_db::get_brand_by_slug(pool, slug)
                .await?
                .ok_or_else(|| anyhow::anyhow!("brand '{slug}' not found"))?;
            vec![brand]
        }
        None => scbdb_db::list_active_brands(pool).await?,
    };

    let brands: Vec<scbdb_db::BrandRow> = all
        .into_iter()
        .filter(|b| {
            if b.shop_url.is_none() {
                tracing::warn!(slug = %b.slug, "skipping brand — shop_url is not set");
                false
            } else {
                true
            }
        })
        .collect();

    Ok(brands)
}

/// Collect full product catalog and variant data from Shopify storefronts,
/// persisting products, variants, and initial price snapshots to the database.
///
/// When `dry_run` is `true` the function prints what would be collected and
/// returns without touching the database.
///
/// # Errors
///
/// Returns an error if the brand filter resolves to nothing, the Shopify
/// client cannot be constructed, or the collection run cannot be created.
/// Per-brand fetch/normalize failures are logged and skipped, not propagated.
pub(crate) async fn run_collect_products(
    pool: &sqlx::PgPool,
    config: &scbdb_core::AppConfig,
    brand_filter: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let brands = load_brands_for_collect(pool, brand_filter).await?;

    if dry_run {
        let slugs: Vec<&str> = brands.iter().map(|b| b.slug.as_str()).collect();
        println!(
            "dry-run: would collect products for {} brands: [{}]",
            brands.len(),
            slugs.join(", ")
        );
        return Ok(());
    }

    let client = scbdb_scraper::ShopifyClient::new(
        config.scraper_request_timeout_secs,
        &config.scraper_user_agent,
    )
    .map_err(|e| anyhow::anyhow!("failed to build Shopify client: {e}"))?;

    let run = scbdb_db::create_collection_run(pool, "products", "cli").await?;
    scbdb_db::start_collection_run(pool, run.id).await?;

    let mut total_records: i64 = 0;
    let brand_count = brands.len();

    for brand in &brands {
        // SAFETY: load_brands_for_collect already filtered out brands without shop_url.
        let shop_url = brand
            .shop_url
            .as_deref()
            .expect("shop_url is Some after filter");

        let raw_products = match client
            .fetch_all_products(shop_url, 250, config.scraper_inter_request_delay_ms)
            .await
        {
            Ok(products) => products,
            Err(e) => {
                let err_string = e.to_string();
                eprintln!(
                    "error: failed to fetch products for {}: {err_string}",
                    brand.slug
                );
                scbdb_db::upsert_collection_run_brand(
                    pool,
                    run.id,
                    brand.id,
                    "failed",
                    None,
                    Some(&err_string),
                )
                .await?;
                continue;
            }
        };

        let mut brand_records: i32 = 0;

        for raw_product in raw_products {
            let normalized = match scbdb_scraper::normalize_product(raw_product, shop_url) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        slug = %brand.slug,
                        error = %e,
                        "skipping product — normalization failed"
                    );
                    continue;
                }
            };

            let product_id = scbdb_db::upsert_product(pool, brand.id, &normalized).await?;

            for variant in &normalized.variants {
                let variant_id = scbdb_db::upsert_variant(pool, product_id, variant).await?;
                scbdb_db::insert_price_snapshot_if_changed(
                    pool,
                    variant_id,
                    Some(run.id),
                    &variant.price,
                    variant.compare_at_price.as_deref(),
                    &variant.currency_code,
                    variant.source_url.as_deref(),
                )
                .await?;
            }

            brand_records = brand_records.saturating_add(1);
        }

        total_records = total_records.saturating_add(i64::from(brand_records));

        scbdb_db::upsert_collection_run_brand(
            pool,
            run.id,
            brand.id,
            "succeeded",
            Some(brand_records),
            None,
        )
        .await?;
    }

    let total_i32 = i32::try_from(total_records).unwrap_or(i32::MAX);
    scbdb_db::complete_collection_run(pool, run.id, total_i32).await?;

    println!("collected {total_records} products across {brand_count} brands");

    Ok(())
}

/// Capture price snapshots for all variants already in the database.
///
/// Fetches the current storefront catalog and records a new `price_snapshots`
/// row for each variant whose price has changed since the last snapshot.
///
/// Per-brand fetch/normalize failures are logged and skipped, not propagated.
///
/// # Errors
///
/// Returns an error if the brand filter resolves to nothing, the Shopify
/// client cannot be constructed, or the collection run cannot be created.
pub(crate) async fn run_collect_pricing(
    pool: &sqlx::PgPool,
    config: &scbdb_core::AppConfig,
    brand_filter: Option<&str>,
) -> anyhow::Result<()> {
    let brands = load_brands_for_collect(pool, brand_filter).await?;

    let client = scbdb_scraper::ShopifyClient::new(
        config.scraper_request_timeout_secs,
        &config.scraper_user_agent,
    )
    .map_err(|e| anyhow::anyhow!("failed to build Shopify client: {e}"))?;

    let run = scbdb_db::create_collection_run(pool, "pricing", "cli").await?;
    scbdb_db::start_collection_run(pool, run.id).await?;

    let mut total_records: i64 = 0;
    let brand_count = brands.len();

    for brand in &brands {
        // SAFETY: load_brands_for_collect already filtered out brands without shop_url.
        let shop_url = brand
            .shop_url
            .as_deref()
            .expect("shop_url is Some after filter");

        let raw_products = match client
            .fetch_all_products(shop_url, 250, config.scraper_inter_request_delay_ms)
            .await
        {
            Ok(products) => products,
            Err(e) => {
                let err_string = e.to_string();
                eprintln!(
                    "error: failed to fetch products for {}: {err_string}",
                    brand.slug
                );
                scbdb_db::upsert_collection_run_brand(
                    pool,
                    run.id,
                    brand.id,
                    "failed",
                    None,
                    Some(&err_string),
                )
                .await?;
                continue;
            }
        };

        let mut brand_records: i32 = 0;

        for raw_product in raw_products {
            let normalized = match scbdb_scraper::normalize_product(raw_product, shop_url) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        slug = %brand.slug,
                        error = %e,
                        "skipping product — normalization failed"
                    );
                    continue;
                }
            };

            let product_id = scbdb_db::upsert_product(pool, brand.id, &normalized).await?;

            for variant in &normalized.variants {
                let variant_id = scbdb_db::upsert_variant(pool, product_id, variant).await?;
                let inserted = scbdb_db::insert_price_snapshot_if_changed(
                    pool,
                    variant_id,
                    Some(run.id),
                    &variant.price,
                    variant.compare_at_price.as_deref(),
                    &variant.currency_code,
                    variant.source_url.as_deref(),
                )
                .await?;
                if inserted {
                    brand_records = brand_records.saturating_add(1);
                }
            }
        }

        total_records = total_records.saturating_add(i64::from(brand_records));

        scbdb_db::upsert_collection_run_brand(
            pool,
            run.id,
            brand.id,
            "succeeded",
            Some(brand_records),
            None,
        )
        .await?;
    }

    let total_i32 = i32::try_from(total_records).unwrap_or(i32::MAX);
    scbdb_db::complete_collection_run(pool, run.id, total_i32).await?;

    println!("captured {total_records} price snapshots across {brand_count} brands");

    Ok(())
}
