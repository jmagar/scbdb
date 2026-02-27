//! Collection command handlers for the CLI.
//!
//! These are called from `main` after the database pool and config are
//! established. Per-brand failures are logged and skipped rather than
//! propagated so a single bad brand does not abort the full run.

mod brand;
mod locations;
mod runner;
mod verify_images;

use clap::Subcommand;

use runner::{load_brands_for_collect, run_collection, BrandOutcome};

pub(crate) use locations::run_collect_locations;

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
    /// Verify stored image URLs (product + brand logo) return HTTP 200
    VerifyImages {
        /// Restrict verification to a specific brand (by slug)
        #[arg(long)]
        brand: Option<String>,
        /// Number of concurrent URL checks
        #[arg(long, default_value_t = 12)]
        concurrency: usize,
    },
    /// Collect store retail locations for all active brands
    Locations {
        /// Restrict collection to a specific brand (by slug)
        #[arg(long)]
        brand: Option<String>,
        /// Preview what would be collected without writing to the database
        #[arg(long)]
        dry_run: bool,
    },
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

    let brand_count = brands.len();
    let totals = run_collection(
        pool,
        config,
        &brands,
        "products",
        |pool, client, config, run_id, b| {
            Box::pin(async move {
                match brand::collect_brand_products(pool, client, config, run_id, b).await {
                    Ok((brand_records, succeeded)) => BrandOutcome::Ok {
                        records: brand_records,
                        extra: 0,
                        succeeded,
                    },
                    Err(e) => BrandOutcome::Err(e),
                }
            })
        },
    )
    .await?;

    println!(
        "collected {} products across {brand_count} brands",
        totals.records
    );
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

    let brand_count = brands.len();
    let totals = run_collection(
        pool,
        config,
        &brands,
        "pricing",
        |pool, client, config, run_id, b| {
            Box::pin(async move {
                match brand::collect_brand_pricing(pool, client, config, run_id, b).await {
                    Ok((brand_records, brand_snapshots, succeeded)) => BrandOutcome::Ok {
                        records: brand_records,
                        extra: brand_snapshots,
                        succeeded,
                    },
                    Err(e) => BrandOutcome::Err(e),
                }
            })
        },
    )
    .await?;

    println!(
        "captured {} price snapshots across {brand_count} brands",
        totals.extra
    );
    Ok(())
}

/// Verify product/brand image URLs currently stored in the database.
///
/// Logs non-200 URLs for cleanup and prints aggregate totals.
pub(crate) async fn run_collect_verify_images(
    pool: &sqlx::PgPool,
    brand_filter: Option<&str>,
    concurrency: usize,
) -> anyhow::Result<()> {
    verify_images::run_collect_verify_images(pool, brand_filter, concurrency).await
}

#[cfg(test)]
#[path = "collect_test.rs"]
mod tests;
