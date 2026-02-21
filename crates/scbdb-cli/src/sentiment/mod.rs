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
#[allow(clippy::too_many_lines)]
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
        let brand_base_url = select_brand_base_url(brand);
        let result = scbdb_sentiment::run_brand_sentiment(
            &sentiment_config,
            &brand.slug,
            &brand.name,
            brand_base_url,
            brand.twitter_handle.as_deref(),
        )
        .await;

        match result {
            Ok(sentiment) => {
                let score = Decimal::from_f32(sentiment.score).unwrap_or(Decimal::ZERO);
                let signal_count = i32::try_from(sentiment.signal_count).unwrap_or(i32::MAX);
                let captured_at = Utc::now();
                let metadata = serde_json::json!({
                    "version": 1,
                    "brand_slug": sentiment.brand_slug,
                    "source_counts": sentiment.source_counts,
                    "top_signals": sentiment.top_signals,
                    "captured_at": captured_at,
                });

                match scbdb_db::insert_sentiment_snapshot(
                    pool,
                    brand.id,
                    captured_at,
                    score,
                    signal_count,
                    metadata,
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

fn select_brand_base_url(brand: &scbdb_db::BrandRow) -> Option<&str> {
    brand
        .domain
        .as_deref()
        .filter(|d| !d.trim().is_empty())
        .or_else(|| brand.shop_url.as_deref().filter(|u| !u.trim().is_empty()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use clap::Parser;
    use sqlx::types::Uuid;

    use crate::{Cli, Commands};

    use super::SentimentCommands;

    fn test_brand_row(domain: Option<&str>, shop_url: Option<&str>) -> scbdb_db::BrandRow {
        scbdb_db::BrandRow {
            id: 1,
            public_id: Uuid::new_v4(),
            name: "Brand".to_string(),
            slug: "brand".to_string(),
            relationship: "owned".to_string(),
            tier: 1,
            domain: domain.map(ToString::to_string),
            shop_url: shop_url.map(ToString::to_string),
            logo_url: None,
            store_locator_url: None,
            notes: None,
            twitter_handle: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn parses_sentiment_collect_defaults() {
        let cli = Cli::try_parse_from(["scbdb-cli", "sentiment", "collect"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Sentiment {
                command: SentimentCommands::Collect {
                    brand: None,
                    dry_run: false,
                }
            })
        ));
    }

    #[test]
    fn parses_sentiment_collect_with_brand() {
        let cli =
            Cli::try_parse_from(["scbdb-cli", "sentiment", "collect", "--brand", "cann"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Sentiment {
                command: SentimentCommands::Collect {
                    brand: Some(ref b),
                    dry_run: false,
                }
            }) if b == "cann"
        ));
    }

    #[test]
    fn parses_sentiment_collect_dry_run() {
        let cli = Cli::try_parse_from(["scbdb-cli", "sentiment", "collect", "--dry-run"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Sentiment {
                command: SentimentCommands::Collect { dry_run: true, .. }
            })
        ));
    }

    #[test]
    fn parses_sentiment_status_no_args() {
        let cli = Cli::try_parse_from(["scbdb-cli", "sentiment", "status"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Sentiment {
                command: SentimentCommands::Status { brand: None }
            })
        ));
    }

    #[test]
    fn parses_sentiment_report_with_brand() {
        let cli =
            Cli::try_parse_from(["scbdb-cli", "sentiment", "report", "--brand", "cann"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Sentiment {
                command: SentimentCommands::Report {
                    brand: Some(ref b)
                }
            }) if b == "cann"
        ));
    }

    #[test]
    fn select_brand_base_url_prefers_domain_then_shop_url_then_none() {
        let brand_with_both = test_brand_row(Some("brand.com"), Some("https://shop.brand.com"));
        assert_eq!(
            super::select_brand_base_url(&brand_with_both),
            Some("brand.com")
        );

        let brand_with_shop_only = test_brand_row(None, Some("https://shop.brand.com"));
        assert_eq!(
            super::select_brand_base_url(&brand_with_shop_only),
            Some("https://shop.brand.com")
        );

        let brand_with_none = test_brand_row(None, None);
        assert_eq!(super::select_brand_base_url(&brand_with_none), None);
    }
}
