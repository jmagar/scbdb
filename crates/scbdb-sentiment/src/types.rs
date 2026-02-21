use std::collections::BTreeMap;

use serde::Serialize;

/// A single piece of content collected for sentiment analysis.
#[derive(Debug, Clone)]
pub struct SentimentSignal {
    /// The text content to score (title + snippet).
    pub text: String,
    /// Source URL of the signal.
    pub url: String,
    /// Source type: `google_news` or `reddit`.
    pub source: String,
    /// Brand slug this signal was collected for.
    pub brand_slug: String,
    /// Lexicon score in [-1.0, 1.0]. Set after scoring.
    pub score: f32,
}

/// Aggregated sentiment result for one brand.
#[derive(Debug, Clone)]
pub struct BrandSentimentResult {
    pub brand_slug: String,
    /// Mean lexicon score across all signals. 0.0 if no signals.
    pub score: f32,
    /// Number of signals collected and scored.
    pub signal_count: usize,
    /// Per-source signal counts used to compute this snapshot.
    pub source_counts: BTreeMap<String, usize>,
    /// Top evidence rows ordered by absolute score contribution.
    pub top_signals: Vec<SignalEvidence>,
}

/// Minimal evidence details for transparency in APIs and UI.
#[derive(Debug, Clone, Serialize)]
pub struct SignalEvidence {
    pub source: String,
    pub url: String,
    pub score: f32,
    pub text_preview: String,
}

/// Configuration for the sentiment pipeline.
#[derive(Debug, Clone)]
pub struct SentimentConfig {
    pub tei_url: String,
    pub qdrant_url: String,
    pub qdrant_collection: String,
    pub reddit_client_id: String,
    pub reddit_client_secret: String,
    pub reddit_user_agent: String,
    /// Optional Twitter/X credentials for bird CLI integration.
    pub twitter_auth_token: Option<String>,
    pub twitter_ct0: Option<String>,
}

impl SentimentConfig {
    /// Build config from environment variables.
    ///
    /// Returns an error string listing any missing variables.
    ///
    /// # Errors
    ///
    /// Returns `Err` if any required env var is not set.
    ///
    /// # Panics
    ///
    /// Does not panic: all `unwrap` calls are guarded by the `missing` check above.
    pub fn from_env() -> Result<Self, String> {
        let tei_url = std::env::var("SENTIMENT_TEI_URL").ok();
        let qdrant_url = std::env::var("SENTIMENT_QDRANT_URL").ok();
        let qdrant_collection = std::env::var("SENTIMENT_QDRANT_COLLECTION").ok();
        let reddit_client_id = std::env::var("REDDIT_CLIENT_ID").ok();
        let reddit_client_secret = std::env::var("REDDIT_CLIENT_SECRET").ok();
        let reddit_user_agent = std::env::var("REDDIT_USER_AGENT").ok();

        if let (
            Some(tei_url),
            Some(qdrant_url),
            Some(qdrant_collection),
            Some(reddit_client_id),
            Some(reddit_client_secret),
            Some(reddit_user_agent),
        ) = (
            tei_url,
            qdrant_url,
            qdrant_collection,
            reddit_client_id,
            reddit_client_secret,
            reddit_user_agent,
        ) {
            Ok(Self {
                tei_url,
                qdrant_url,
                qdrant_collection,
                reddit_client_id,
                reddit_client_secret,
                reddit_user_agent,
                twitter_auth_token: std::env::var("TWITTER_AUTH_TOKEN").ok(),
                twitter_ct0: std::env::var("TWITTER_CT0").ok(),
            })
        } else {
            let mut missing = Vec::new();
            if std::env::var("SENTIMENT_TEI_URL").is_err() {
                missing.push("SENTIMENT_TEI_URL");
            }
            if std::env::var("SENTIMENT_QDRANT_URL").is_err() {
                missing.push("SENTIMENT_QDRANT_URL");
            }
            if std::env::var("SENTIMENT_QDRANT_COLLECTION").is_err() {
                missing.push("SENTIMENT_QDRANT_COLLECTION");
            }
            if std::env::var("REDDIT_CLIENT_ID").is_err() {
                missing.push("REDDIT_CLIENT_ID");
            }
            if std::env::var("REDDIT_CLIENT_SECRET").is_err() {
                missing.push("REDDIT_CLIENT_SECRET");
            }
            if std::env::var("REDDIT_USER_AGENT").is_err() {
                missing.push("REDDIT_USER_AGENT");
            }
            Err(format!(
                "missing sentiment env vars: {}",
                missing.join(", ")
            ))
        }
    }
}
