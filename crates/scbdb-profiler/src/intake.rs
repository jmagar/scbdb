//! Signal intake pipeline: collect, embed, and persist brand signals.
//!
//! Orchestrates the full signal collection pipeline for a single brand:
//! 1. Collect signals from RSS feeds and `YouTube` (Twitter is optional/best-effort)
//! 2. For each signal, derive a deterministic Qdrant point ID and attempt embedding
//! 3. Upsert the signal into the database via [`scbdb_db::upsert_brand_signal`]
//! 4. Return a [`BrandProfileRunResult`] with counts and per-collector errors

use crate::{error::ProfilerError, types::BrandProfileRunResult, types::CollectedSignal};
use sqlx::PgPool;
use tracing::debug;

/// Configuration for the intake pipeline.
#[derive(Debug, Clone)]
pub struct IntakeConfig {
    /// HTTP client to share across all requests (reuse connections).
    pub client: reqwest::Client,
    /// TEI base URL (e.g., `"http://localhost:8080"`).
    pub tei_url: String,
    /// Optional `YouTube` Data API v3 key.
    pub youtube_api_key: Option<String>,
}

/// Ingest signals for a single brand.
///
/// Runs all enabled collectors (RSS feeds from brand's domains, `YouTube` channel,
/// Twitter profile), embeds each signal, and upserts into the database.
///
/// Individual collector failures are captured in [`BrandProfileRunResult::errors`]
/// rather than propagated -- the pipeline continues past non-fatal failures.
///
/// # Errors
///
/// Returns [`ProfilerError::Other`] only on fatal orchestration errors (e.g. if
/// the entire pipeline cannot start). Collector and DB errors per-signal are
/// recorded in the result's `errors` vec.
pub async fn ingest_signals(
    pool: &PgPool,
    config: &IntakeConfig,
    brand_id: i64,
    feed_urls: &[String],
    youtube_channel_id: Option<&str>,
    twitter_handle: Option<&str>,
) -> Result<BrandProfileRunResult, ProfilerError> {
    let mut all_signals: Vec<CollectedSignal> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // 1. Collect from RSS feeds
    for url in feed_urls {
        match crate::rss::crawl_feed(&config.client, brand_id, url).await {
            Ok(signals) => all_signals.extend(signals),
            Err(e) => errors.push(format!("RSS {url}: {e}")),
        }
    }

    // 2. Collect from YouTube (requires both channel_id AND api_key)
    if let (Some(channel_id), Some(api_key)) = (youtube_channel_id, &config.youtube_api_key) {
        match crate::youtube::collect_channel_signals(
            &config.client,
            brand_id,
            channel_id,
            api_key,
            50,
        )
        .await
        {
            Ok(signals) => all_signals.extend(signals),
            Err(e) => errors.push(format!("YouTube {channel_id}: {e}")),
        }
    }

    // 3. Collect from Twitter (optional, best-effort)
    if let Some(handle) = twitter_handle {
        match crate::twitter::collect_profile_signals(brand_id, handle, 50).await {
            Ok(signals) => all_signals.extend(signals),
            Err(e) => errors.push(format!("Twitter @{handle}: {e}")),
        }
    }

    let signals_collected = all_signals.len();
    let mut signals_upserted = 0;

    // 4. Embed and upsert each signal
    for signal in &all_signals {
        let text_to_embed = build_embed_text(signal);

        // Deterministic content key for Qdrant point ID derivation.
        // When no external_id, source_url, or title is available, generate a
        // unique fallback from a UUID to avoid ID collisions across signals.
        let fallback_key;
        let content_key = match signal
            .external_id
            .as_deref()
            .or(signal.source_url.as_deref())
            .or(signal.title.as_deref())
        {
            Some(key) => key,
            None => {
                fallback_key = uuid::Uuid::new_v4().to_string();
                &fallback_key
            }
        };

        let qdrant_point_id = crate::embedder::signal_point_id(content_key);

        // Attempt embedding via TEI (best-effort; failure doesn't block DB upsert)
        if !config.tei_url.is_empty() {
            match crate::embedder::embed_text(&config.client, &config.tei_url, &text_to_embed).await
            {
                Ok(_embedding) => {
                    // TODO: send embedding vector to Qdrant once client is wired
                    debug!(brand_id, content_key, "embedded signal via TEI");
                }
                Err(e) => {
                    debug!(brand_id, content_key, error = %e, "TEI embedding failed (non-fatal)");
                }
            }
        }

        // Upsert into DB
        let new_signal = scbdb_db::NewBrandSignal {
            brand_id,
            signal_type: signal.signal_type.as_str(),
            source_platform: signal.source_platform.as_deref(),
            source_url: signal.source_url.as_deref(),
            external_id: signal.external_id.as_deref(),
            title: signal.title.as_deref(),
            summary: signal.summary.as_deref(),
            content: None, // full content extraction is a future enhancement
            image_url: signal.image_url.as_deref(),
            qdrant_point_id: Some(&qdrant_point_id),
            published_at: signal.published_at,
        };

        match scbdb_db::upsert_brand_signal(pool, &new_signal).await {
            Ok(_id) => signals_upserted += 1,
            Err(e) => errors.push(format!("DB upsert: {e}")),
        }
    }

    Ok(BrandProfileRunResult {
        brand_id,
        signals_collected,
        signals_upserted,
        errors,
    })
}

/// Build text for embedding from a signal's title and summary.
///
/// Concatenates title and summary with a double newline separator when both
/// are present. Returns whichever is available, or an empty string if neither.
fn build_embed_text(signal: &CollectedSignal) -> String {
    match (&signal.title, &signal.summary) {
        (Some(title), Some(summary)) => format!("{title}\n\n{summary}"),
        (Some(title), None) => title.clone(),
        (None, Some(summary)) => summary.clone(),
        (None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal(title: Option<&str>, summary: Option<&str>) -> CollectedSignal {
        CollectedSignal {
            brand_id: 1,
            signal_type: "article".to_string(),
            source_platform: None,
            source_url: None,
            external_id: None,
            title: title.map(str::to_string),
            summary: summary.map(str::to_string),
            image_url: None,
            view_count: None,
            like_count: None,
            comment_count: None,
            share_count: None,
            published_at: None,
        }
    }

    #[test]
    fn build_embed_text_both() {
        let s = make_signal(Some("Title"), Some("Summary text"));
        assert_eq!(build_embed_text(&s), "Title\n\nSummary text");
    }

    #[test]
    fn build_embed_text_title_only() {
        let s = make_signal(Some("Title only"), None);
        assert_eq!(build_embed_text(&s), "Title only");
    }

    #[test]
    fn build_embed_text_summary_only() {
        let s = make_signal(None, Some("Summary only"));
        assert_eq!(build_embed_text(&s), "Summary only");
    }

    #[test]
    fn build_embed_text_empty() {
        let s = make_signal(None, None);
        assert_eq!(build_embed_text(&s), "");
    }

    #[test]
    fn intake_config_is_cloneable() {
        let config = IntakeConfig {
            client: reqwest::Client::new(),
            tei_url: "http://localhost:8080".to_string(),
            youtube_api_key: Some("test-key".to_string()),
        };
        let cloned = config.clone();
        assert_eq!(cloned.tei_url, "http://localhost:8080");
        assert_eq!(cloned.youtube_api_key, Some("test-key".to_string()));
    }
}
