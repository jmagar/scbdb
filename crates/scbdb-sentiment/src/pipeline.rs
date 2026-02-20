//! Sentiment pipeline orchestration.

use crate::embeddings::TeiClient;
use crate::error::SentimentError;
use crate::scorer::lexicon_score;
use crate::sources::collect_signals;
use crate::types::{BrandSentimentResult, SentimentConfig, SentimentSignal};
use crate::vector_store::QdrantClient;

/// Run the full sentiment pipeline for one brand.
///
/// 1. Collect signals from all sources (Google News RSS + Reddit).
/// 2. Embed signal texts via TEI (batch size 64).
/// 3. Deduplicate by URL using Qdrant — skip signals already stored.
/// 4. Upsert new signals (with embeddings and scores) to Qdrant.
/// 5. Score all signals with the lexicon scorer.
/// 6. Return the aggregated `BrandSentimentResult`.
///
/// Empty signal sets produce a neutral score of `0.0`.
///
/// # Errors
///
/// Returns [`SentimentError`] if TEI or Qdrant calls fail fatally.
/// Individual source failures are logged and skipped (see [`collect_signals`]).
pub async fn run_brand_sentiment(
    config: &SentimentConfig,
    brand_slug: &str,
    brand_name: &str,
) -> Result<BrandSentimentResult, SentimentError> {
    let tei = TeiClient::new(&config.tei_url);
    let qdrant = QdrantClient::new(&config.qdrant_url, &config.qdrant_collection);

    // Ensure Qdrant collection exists before any operations.
    qdrant.ensure_collection().await?;

    // Step 1: Collect signals from all sources.
    let mut signals = collect_signals(config, brand_slug, brand_name).await;

    if signals.is_empty() {
        tracing::info!(
            brand = brand_slug,
            "no signals collected — returning neutral score"
        );
        return Ok(BrandSentimentResult {
            brand_slug: brand_slug.to_string(),
            score: 0.0,
            signal_count: 0,
        });
    }

    // Step 2: Embed all signal texts.
    let texts: Vec<&str> = signals.iter().map(|s| s.text.as_str()).collect();
    let embeddings = tei.embed(&texts).await?;

    // Step 3 + 4: Dedup by URL; upsert new signals to Qdrant; score each signal.
    let expected_signal_count = signals.len();
    let mut scored_signals: Vec<SentimentSignal> = Vec::new();

    for (mut signal, embedding) in signals.drain(..).zip(embeddings) {
        // Score the signal.
        signal.score = lexicon_score(&signal.text);

        // Check for duplicates; skip Qdrant write if already stored.
        match qdrant.signal_exists(&signal.url).await {
            Ok(true) => {
                tracing::debug!(url = %signal.url, "signal already in Qdrant, skipping upsert");
            }
            Ok(false) => {
                if let Err(e) = qdrant.upsert_signal(&signal, embedding).await {
                    tracing::warn!(url = %signal.url, error = %e, "Qdrant upsert failed");
                }
            }
            Err(e) => {
                tracing::warn!(url = %signal.url, error = %e, "Qdrant existence check failed");
            }
        }

        scored_signals.push(signal);
    }

    // Warn if TEI returned fewer embeddings than expected (TEI contract violation).
    if scored_signals.len() != expected_signal_count {
        tracing::warn!(
            brand = brand_slug,
            expected = expected_signal_count,
            got = scored_signals.len(),
            "TEI returned fewer embeddings than expected; some signals were dropped"
        );
    }

    // Step 5: Aggregate.
    let score = if scored_signals.is_empty() {
        0.0
    } else {
        #[allow(clippy::cast_precision_loss)]
        let denom = scored_signals.len() as f32;
        let sum: f32 = scored_signals.iter().map(|s| s.score).sum();
        sum / denom
    };

    Ok(BrandSentimentResult {
        brand_slug: brand_slug.to_string(),
        score,
        signal_count: scored_signals.len(),
    })
}
