//! Collection command handlers for the CLI.
//!
//! These are called from `main` after the database pool and config are
//! established. Per-brand failures are logged and skipped rather than
//! propagated so a single bad brand does not abort the full run.

mod brand;

use clap::Subcommand;

use crate::fail_run_best_effort;

/// Sub-commands available under `collect`.
#[derive(Debug, Subcommand)]
pub enum CollectCommands {
    /// Collect full product catalog and variant data from all active brands
    Products {
        /// Restrict collection to a specific brand (by slug)
        #[arg(long)]
        brand: Option<String>,

        /// Preview what would be collected without writing to the database
        #[arg(long)]
        dry_run: bool,
    },
    /// Capture pricing snapshots for products already in the database
    Pricing {
        /// Restrict snapshots to a specific brand (by slug)
        #[arg(long)]
        brand: Option<String>,
    },
}

/// Load the brands to process for a collect run.
///
/// If `brand_filter` is `Some(slug)`, fetches that single brand and returns an
/// error if not found or if `shop_url` is `None`. If `None`, returns all
/// active brands, filtering out those without a `shop_url` (with a warning).
pub(crate) async fn load_brands_for_collect(
    pool: &sqlx::PgPool,
    brand_filter: Option<&str>,
) -> anyhow::Result<Vec<scbdb_db::BrandRow>> {
    if let Some(slug) = brand_filter {
        let brand = scbdb_db::get_brand_by_slug(pool, slug)
            .await?
            .ok_or_else(|| anyhow::anyhow!("brand '{slug}' not found"))?;
        if brand.shop_url.is_none() {
            anyhow::bail!(
                "brand '{slug}' exists but has no shop_url configured; update config/brands.yaml"
            );
        }
        // Single-brand path: already validated shop_url above, no filter needed.
        Ok(vec![brand])
    } else {
        let all = scbdb_db::list_active_brands(pool).await?;
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
    if brands.is_empty() {
        println!("no eligible brands found for product collection; skipping run creation");
        return Ok(());
    }

    if dry_run {
        let slugs: Vec<&str> = brands.iter().map(|b| b.slug.as_str()).collect();
        println!(
            "dry-run: would collect products for {} brands: [{}]",
            brands.len(),
            slugs.join(", ")
        );
        return Ok(());
    }

    let client = brand::build_shopify_client(config)?;

    let run = scbdb_db::create_collection_run(pool, "products", "cli").await?;
    if let Err(e) = scbdb_db::start_collection_run(pool, run.id).await {
        fail_run_best_effort(pool, run.id, "products", format!("{e:#}")).await;
        return Err(e.into());
    }

    let mut total_records: i32 = 0;
    let mut failed_brands: usize = 0;
    let brand_count = brands.len();

    for b in &brands {
        match brand::collect_brand_products(pool, &client, config, run.id, b).await {
            Ok((brand_records, succeeded)) => {
                total_records = total_records.saturating_add(brand_records);
                if !succeeded {
                    failed_brands += 1;
                }
            }
            Err(e) => {
                tracing::error!(brand = %b.slug, error = %e, "unexpected error collecting products");
                failed_brands += 1;
            }
        }
    }

    if failed_brands > 0 {
        tracing::warn!(
            failed_brands,
            total_brands = brand_count,
            "some brands failed during collection"
        );
    }

    if failed_brands == brand_count {
        let message = format!("all {failed_brands} brands failed collection");
        fail_run_best_effort(pool, run.id, "products", message.clone()).await;
        anyhow::bail!("{message}");
    }

    if let Err(err) = scbdb_db::complete_collection_run(pool, run.id, total_records).await {
        let message = format!("{err:#}");
        fail_run_best_effort(pool, run.id, "products", message).await;
        return Err(err.into());
    }
    println!("collected {total_records} products across {brand_count} brands");
    Ok(())
}

/// Capture price snapshots for all brands' Shopify storefronts.
///
/// Fetches the current storefront catalog, upserts any new products/variants
/// encountered, and records a new `price_snapshots` row for each variant
/// whose price has changed since the last snapshot. New products are persisted
/// as a side effect so that pricing data is never lost when a brand adds
/// products between collection runs.
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
    if brands.is_empty() {
        println!("no eligible brands found for pricing collection; skipping run creation");
        return Ok(());
    }

    let client = brand::build_shopify_client(config)?;

    let run = scbdb_db::create_collection_run(pool, "pricing", "cli").await?;
    if let Err(e) = scbdb_db::start_collection_run(pool, run.id).await {
        fail_run_best_effort(pool, run.id, "pricing", format!("{e:#}")).await;
        return Err(e.into());
    }

    // `records_processed` is consistently the number of products processed.
    let mut total_records: i32 = 0;
    let mut total_snapshots: i32 = 0;
    let mut failed_brands: usize = 0;
    let brand_count = brands.len();

    for b in &brands {
        match brand::collect_brand_pricing(pool, &client, config, run.id, b).await {
            Ok((brand_records, brand_snapshots, succeeded)) => {
                total_records = total_records.saturating_add(brand_records);
                total_snapshots = total_snapshots.saturating_add(brand_snapshots);
                if !succeeded {
                    failed_brands += 1;
                }
            }
            Err(e) => {
                tracing::error!(brand = %b.slug, error = %e, "unexpected error collecting pricing");
                failed_brands += 1;
            }
        }
    }

    if failed_brands > 0 {
        tracing::warn!(
            failed_brands,
            total_brands = brand_count,
            "some brands failed during collection"
        );
    }

    if failed_brands == brand_count {
        let message = format!("all {failed_brands} brands failed collection");
        fail_run_best_effort(pool, run.id, "pricing", message.clone()).await;
        anyhow::bail!("{message}");
    }

    if let Err(err) = scbdb_db::complete_collection_run(pool, run.id, total_records).await {
        let message = format!("{err:#}");
        fail_run_best_effort(pool, run.id, "pricing", message).await;
        return Err(err.into());
    }
    println!("captured {total_snapshots} price snapshots across {brand_count} brands");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Insert a minimal brand row for test purposes.
    ///
    /// When `shop_url` is `None` the brand is inserted without a shop URL.
    async fn insert_test_brand(pool: &sqlx::PgPool, slug: &str, shop_url: Option<&str>) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "INSERT INTO brands (name, slug, relationship, tier, shop_url, is_active) \
             VALUES ($1, $2, 'portfolio', 1, $3, true) RETURNING id",
        )
        .bind(format!("Test Brand {slug}"))
        .bind(slug)
        .bind(shop_url)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("insert_test_brand failed for slug '{slug}': {e}"))
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn load_brands_unknown_slug_returns_error(pool: sqlx::PgPool) {
        // Insert a brand with a different slug so the table isn't empty.
        insert_test_brand(&pool, "existing-brand", Some("https://existing.com")).await;

        let result = load_brands_for_collect(&pool, Some("nonexistent")).await;

        let err = result.expect_err("expected Err for unknown slug");
        let msg = format!("{err}");
        assert!(
            msg.contains("not found"),
            "error should mention 'not found', got: {msg}"
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn load_brands_known_slug_null_shop_url_returns_error(pool: sqlx::PgPool) {
        insert_test_brand(&pool, "no-shop", None).await;

        let result = load_brands_for_collect(&pool, Some("no-shop")).await;

        let err = result.expect_err("expected Err for brand with null shop_url");
        let msg = format!("{err}");
        assert!(
            msg.contains("no shop_url"),
            "error should mention 'no shop_url', got: {msg}"
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn load_brands_all_filters_out_null_shop_url(pool: sqlx::PgPool) {
        insert_test_brand(&pool, "has-shop", Some("https://has-shop.com")).await;
        insert_test_brand(&pool, "no-shop-all", None).await;

        let brands = load_brands_for_collect(&pool, None)
            .await
            .expect("load_brands_for_collect should succeed");

        assert_eq!(
            brands.len(),
            1,
            "only the brand with shop_url should be returned"
        );
        assert_eq!(brands[0].slug, "has-shop");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn run_collect_products_dry_run_writes_zero_db_rows(pool: sqlx::PgPool) {
        insert_test_brand(&pool, "dry-run-brand", Some("https://dry-run.com")).await;

        let config = scbdb_core::AppConfig {
            database_url: String::new(),
            env: scbdb_core::Environment::Test,
            bind_addr: "0.0.0.0:3000".parse().unwrap(),
            log_level: "info".to_string(),
            brands_path: std::path::PathBuf::from("config/brands.yaml"),
            api_key_hash_salt: None,
            legiscan_api_key: None,
            db_max_connections: 10,
            db_min_connections: 1,
            db_acquire_timeout_secs: 10,
            scraper_request_timeout_secs: 30,
            scraper_user_agent: "scbdb/0.1 (test)".to_string(),
            scraper_max_concurrent_brands: 1,
            scraper_inter_request_delay_ms: 0,
            scraper_max_retries: 3,
            scraper_retry_backoff_base_secs: 5,
        };

        let result = run_collect_products(&pool, &config, None, true).await;
        assert!(
            result.is_ok(),
            "dry-run should return Ok(()), got: {result:?}"
        );

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_runs")
            .fetch_one(&pool)
            .await
            .expect("count query failed");

        assert_eq!(count, 0, "dry-run must not create any collection_runs rows");
    }
}
