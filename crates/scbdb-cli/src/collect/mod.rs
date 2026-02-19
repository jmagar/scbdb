//! Collection command handlers for the CLI.
//!
//! These are called from `main` after the database pool and config are
//! established. Per-brand failures are logged and skipped rather than
//! propagated so a single bad brand does not abort the full run.

mod brand;

use clap::Subcommand;

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
            if brand.shop_url.is_none() {
                anyhow::bail!(
                    "brand '{slug}' exists but has no shop_url configured; update config/brands.yaml"
                );
            }
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
    scbdb_db::start_collection_run(pool, run.id).await?;

    let mut total_records: i32 = 0;
    let brand_count = brands.len();

    let collect_result: anyhow::Result<()> = async {
        for b in &brands {
            let brand_records =
                brand::collect_brand_products(pool, &client, config, run.id, b).await?;
            total_records = total_records.saturating_add(brand_records);
        }
        Ok(())
    }
    .await;

    match collect_result {
        Ok(()) => {
            if let Err(err) = scbdb_db::complete_collection_run(pool, run.id, total_records).await {
                let message = format!("{err:#}");
                brand::fail_run_best_effort(pool, run.id, "collection", message).await;
                return Err(err.into());
            }
            println!("collected {total_records} products across {brand_count} brands");
            Ok(())
        }
        Err(err) => {
            brand::fail_run_best_effort(pool, run.id, "collection", format!("{err:#}")).await;
            Err(err)
        }
    }
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
    scbdb_db::start_collection_run(pool, run.id).await?;

    // `records_processed` is consistently the number of products processed.
    let mut total_records: i32 = 0;
    let mut total_snapshots: i32 = 0;
    let brand_count = brands.len();

    let collect_result: anyhow::Result<()> = async {
        for b in &brands {
            let (brand_records, brand_snapshots) =
                brand::collect_brand_pricing(pool, &client, config, run.id, b).await?;
            total_records = total_records.saturating_add(brand_records);
            total_snapshots = total_snapshots.saturating_add(brand_snapshots);
        }
        Ok(())
    }
    .await;

    match collect_result {
        Ok(()) => {
            if let Err(err) = scbdb_db::complete_collection_run(pool, run.id, total_records).await {
                let message = format!("{err:#}");
                brand::fail_run_best_effort(pool, run.id, "pricing", message).await;
                return Err(err.into());
            }
            println!("captured {total_snapshots} price snapshots across {brand_count} brands");
            Ok(())
        }
        Err(err) => {
            brand::fail_run_best_effort(pool, run.id, "pricing", format!("{err:#}")).await;
            Err(err)
        }
    }
}
