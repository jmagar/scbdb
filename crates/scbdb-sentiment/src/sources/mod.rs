//! Signal source abstractions.

mod reddit;
mod rss;

pub(crate) use reddit::RedditClient;
pub(crate) use rss::fetch_google_news_rss;

use crate::types::{SentimentConfig, SentimentSignal};

/// Collect signals from all sources for a brand.
///
/// Continues past individual source failures, logging warnings.
/// Returns an empty `Vec` if all sources fail.
pub(crate) async fn collect_signals(
    config: &SentimentConfig,
    brand_slug: &str,
    brand_name: &str,
) -> Vec<SentimentSignal> {
    let mut signals = Vec::new();

    // Google News RSS
    match fetch_google_news_rss(brand_slug, brand_name).await {
        Ok(rss_signals) => {
            tracing::debug!(
                brand = brand_slug,
                count = rss_signals.len(),
                "collected RSS signals"
            );
            signals.extend(rss_signals);
        }
        Err(e) => {
            tracing::warn!(brand = brand_slug, error = %e, "Google News RSS fetch failed");
        }
    }

    // Reddit
    match RedditClient::new(config).await {
        Ok(client) => match client.search_brand_mentions(brand_slug, brand_name).await {
            Ok(reddit_signals) => {
                tracing::debug!(
                    brand = brand_slug,
                    count = reddit_signals.len(),
                    "collected Reddit signals"
                );
                signals.extend(reddit_signals);
            }
            Err(e) => {
                tracing::warn!(brand = brand_slug, error = %e, "Reddit search failed");
            }
        },
        Err(e) => {
            tracing::warn!(brand = brand_slug, error = %e, "Reddit client creation failed");
        }
    }

    signals
}
