//! Sentiment pipeline command handlers for the CLI.

mod query;

use chrono::Utc;
use clap::Subcommand;
use rust_decimal::prelude::*;

use crate::fail_run_best_effort;

pub(crate) use query::{run_sentiment_report, run_sentiment_status};

/// Sub-commands available under `sentiment`.
#[derive(Debug, Subcommand)]
pub enum SentimentCommands {
    /// Collect sentiment signals and score all active brands
    Collect {
        /// Restrict collection to a specific brand (by slug)
        #[arg(long)]
        brand: Option<String>,

        /// Preview what would be collected without writing to the database or Qdrant
        #[arg(long)]
        dry_run: bool,
    },
    /// Show recent sentiment scores for brands
    Status {
        /// Filter to a specific brand (by slug)
        #[arg(long)]
        brand: Option<String>,
    },
    /// Generate a markdown sentiment report
    Report {
        /// Filter to a specific brand (by slug)
        #[arg(long)]
        brand: Option<String>,
    },
}

/// Load brands eligible for sentiment collection.
///
/// Same logic as collect: if a `brand_filter` is given, look up that single brand.
/// Otherwise load all active brands. Unlike product collection, brands do not
/// need a `shop_url` to be eligible for sentiment collection.
///
/// # Errors
///
/// Returns an error if the brand filter slug is not found.
pub(crate) async fn load_brands_for_sentiment(
    pool: &sqlx::PgPool,
    brand_filter: Option<&str>,
) -> anyhow::Result<Vec<scbdb_db::BrandRow>> {
    if let Some(slug) = brand_filter {
        let brand = scbdb_db::get_brand_by_slug(pool, slug)
            .await?
            .ok_or_else(|| anyhow::anyhow!("brand '{slug}' not found"))?;
        Ok(vec![brand])
    } else {
        let brands = scbdb_db::list_active_brands(pool).await?;
        Ok(brands)
    }
}

/// Run sentiment collection for all (or one) brand(s).
///
/// Collects signals from Google News RSS and Reddit, embeds them via TEI,
/// deduplicates in Qdrant, scores with the lexicon, and persists a
/// `sentiment_snapshots` row per brand. A collection run tracks overall progress.
///
/// When `dry_run` is `true`, prints the brand list and returns without touching
/// the database or Qdrant.
///
/// # Errors
///
/// Returns an error if no brands are found, the sentiment config is missing env vars,
/// or the collection run cannot be created. Per-brand failures are logged and skipped.
pub(crate) async fn run_sentiment_collect(
    pool: &sqlx::PgPool,
    brand_filter: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let brands = load_brands_for_sentiment(pool, brand_filter).await?;

    if brands.is_empty() {
        println!("no active brands found for sentiment collection; skipping run creation");
        return Ok(());
    }

    if dry_run {
        let slugs: Vec<&str> = brands.iter().map(|b| b.slug.as_str()).collect();
        println!(
            "dry-run: would collect sentiment for {} brands: [{}]",
            brands.len(),
            slugs.join(", ")
        );
        return Ok(());
    }

    let sentiment_config =
        scbdb_sentiment::SentimentConfig::from_env().map_err(|e| anyhow::anyhow!("{e}"))?;

    let run = scbdb_db::create_collection_run(pool, "sentiment", "cli").await?;
    if let Err(e) = scbdb_db::start_collection_run(pool, run.id).await {
        fail_run_best_effort(pool, run.id, "sentiment", format!("{e:#}")).await;
        return Err(e.into());
    }

    let mut total_signals: i32 = 0;
    let mut failed_brands: usize = 0;
    let brand_count = brands.len();

    for brand in &brands {
        let result =
            scbdb_sentiment::run_brand_sentiment(&sentiment_config, &brand.slug, &brand.name).await;

        match result {
            Ok(sentiment) => {
                let score = Decimal::from_f32(sentiment.score).unwrap_or(Decimal::ZERO);
                let signal_count = i32::try_from(sentiment.signal_count).unwrap_or(i32::MAX);
                let captured_at = Utc::now();

                match scbdb_db::insert_sentiment_snapshot(
                    pool,
                    brand.id,
                    captured_at,
                    score,
                    signal_count,
                    serde_json::json!({}),
                )
                .await
                {
                    Ok(_) => {
                        tracing::info!(
                            brand = %brand.slug,
                            score = %sentiment.score,
                            signals = sentiment.signal_count,
                            "sentiment snapshot recorded"
                        );
                        total_signals = total_signals.saturating_add(signal_count);
                    }
                    Err(e) => {
                        tracing::error!(
                            brand = %brand.slug,
                            error = %e,
                            "failed to save sentiment snapshot"
                        );
                        failed_brands += 1;
                    }
                }
            }
            Err(e) => {
                tracing::error!(brand = %brand.slug, error = %e, "sentiment collection failed");
                failed_brands += 1;
            }
        }
    }

    if failed_brands > 0 {
        tracing::warn!(
            failed_brands,
            total_brands = brand_count,
            "some brands failed during sentiment collection"
        );
    }

    if failed_brands == brand_count {
        let message = format!("all {failed_brands} brands failed sentiment collection");
        fail_run_best_effort(pool, run.id, "sentiment", message.clone()).await;
        anyhow::bail!("{message}");
    }

    if let Err(err) = scbdb_db::complete_collection_run(pool, run.id, total_signals).await {
        let message = format!("{err:#}");
        fail_run_best_effort(pool, run.id, "sentiment", message).await;
        return Err(err.into());
    }

    println!(
        "sentiment collection complete: {} brands processed, {} signals scored",
        brand_count - failed_brands,
        total_signals
    );
    Ok(())
}
