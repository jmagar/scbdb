//! Shared orchestration types and helpers for collection runs.
//!
//! Provides `BrandOutcome`, `CollectionTotals`, `load_brands_for_collect`,
//! and the `run_collection` skeleton used by products and pricing handlers.

use std::future::Future;
use std::pin::Pin;

use futures::stream::{self, StreamExt};

use crate::fail_run_best_effort;

/// Outcome of processing a single brand: primary `records`, secondary `extra`
/// count (e.g. price snapshots), and a `succeeded` flag for partial-failure
/// tracking. `Err` wraps unexpected per-brand errors.
pub(super) enum BrandOutcome {
    Ok {
        records: i32,
        extra: i32,
        succeeded: bool,
    },
    Err(anyhow::Error),
}

/// Aggregated totals returned by [`run_collection`]: primary `records` count
/// and secondary `extra` count (unused for product runs).
pub(super) struct CollectionTotals {
    pub records: i32,
    pub extra: i32,
}

/// Load the brands to process for a collect run.
///
/// If `brand_filter` is `Some(slug)`, fetches that single brand and returns an
/// error if not found or if `shop_url` is `None`. If `None`, returns all
/// active brands, filtering out those without a `shop_url` (with a warning).
pub(super) async fn load_brands_for_collect(
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

/// Shared orchestration skeleton for collection runs (create → start → loop
/// → complete/fail). `process_brand` receives `(pool, client, config,
/// run_id, brand)` and returns a `BrandOutcome`.
pub(super) async fn run_collection<F>(
    pool: &sqlx::PgPool,
    config: &scbdb_core::AppConfig,
    brands: &[scbdb_db::BrandRow],
    collection_type: &'static str,
    process_brand: F,
) -> anyhow::Result<CollectionTotals>
where
    F: for<'a> Fn(
        &'a sqlx::PgPool,
        &'a scbdb_scraper::ShopifyClient,
        &'a scbdb_core::AppConfig,
        i64,
        &'a scbdb_db::BrandRow,
    ) -> Pin<Box<dyn Future<Output = BrandOutcome> + 'a>>,
{
    let client = super::brand::build_shopify_client(config)?;

    let run = scbdb_db::create_collection_run(pool, collection_type, "cli").await?;
    if let Err(e) = scbdb_db::start_collection_run(pool, run.id).await {
        fail_run_best_effort(pool, run.id, collection_type, format!("{e:#}")).await;
        return Err(e.into());
    }

    let mut total_records: i32 = 0;
    let mut total_extra: i32 = 0;
    let mut failed_brands: usize = 0;
    let brand_count = brands.len();

    let max_concurrent = config.scraper_max_concurrent_brands.max(1);

    let results: Vec<(&scbdb_db::BrandRow, BrandOutcome)> = stream::iter(brands)
        .map(|b| {
            let fut = process_brand(pool, &client, config, run.id, b);
            async move { (b, fut.await) }
        })
        .buffer_unordered(max_concurrent)
        .collect()
        .await;

    for (b, outcome) in &results {
        match outcome {
            BrandOutcome::Ok {
                records,
                extra,
                succeeded,
            } => {
                total_records = total_records.saturating_add(*records);
                total_extra = total_extra.saturating_add(*extra);
                if !succeeded {
                    failed_brands += 1;
                }
            }
            BrandOutcome::Err(e) => {
                tracing::error!(
                    brand = %b.slug,
                    error = %e,
                    "unexpected error collecting {collection_type}"
                );
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
        fail_run_best_effort(pool, run.id, collection_type, message.clone()).await;
        anyhow::bail!("{message}");
    }

    if let Err(err) = scbdb_db::complete_collection_run(pool, run.id, total_records).await {
        let message = format!("{err:#}");
        fail_run_best_effort(pool, run.id, collection_type, message).await;
        return Err(err.into());
    }

    Ok(CollectionTotals {
        records: total_records,
        extra: total_extra,
    })
}
