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
        let mut missing = Vec::new();

        let get = |key: &str| -> Option<String> { std::env::var(key).ok() };

        let tei_url = get("SENTIMENT_TEI_URL");
        let qdrant_url = get("SENTIMENT_QDRANT_URL");
        let qdrant_collection = get("SENTIMENT_QDRANT_COLLECTION");
        let reddit_client_id = get("REDDIT_CLIENT_ID");
        let reddit_client_secret = get("REDDIT_CLIENT_SECRET");
        let reddit_user_agent = get("REDDIT_USER_AGENT");

        if tei_url.is_none() {
            missing.push("SENTIMENT_TEI_URL");
        }
        if qdrant_url.is_none() {
            missing.push("SENTIMENT_QDRANT_URL");
        }
        if qdrant_collection.is_none() {
            missing.push("SENTIMENT_QDRANT_COLLECTION");
        }
        if reddit_client_id.is_none() {
            missing.push("REDDIT_CLIENT_ID");
        }
        if reddit_client_secret.is_none() {
            missing.push("REDDIT_CLIENT_SECRET");
        }
        if reddit_user_agent.is_none() {
            missing.push("REDDIT_USER_AGENT");
        }

        if !missing.is_empty() {
            return Err(format!(
                "missing sentiment env vars: {}",
                missing.join(", ")
            ));
        }

        Ok(Self {
            tei_url: tei_url.unwrap(),
            qdrant_url: qdrant_url.unwrap(),
            qdrant_collection: qdrant_collection.unwrap(),
            reddit_client_id: reddit_client_id.unwrap(),
            reddit_client_secret: reddit_client_secret.unwrap(),
            reddit_user_agent: reddit_user_agent.unwrap(),
        })
    }
}
