//! Signal source abstractions.

mod bing_rss;
mod brand_newsroom;
mod reddit;
mod rss;
mod twitter;
mod yahoo_rss;

pub(crate) use bing_rss::fetch_bing_news_rss;
pub(crate) use brand_newsroom::fetch_brand_newsroom_signals;
pub(crate) use reddit::RedditClient;
pub(crate) use rss::fetch_google_news_rss;
use twitter::fetch_twitter_brand_and_replies;
use twitter::fetch_twitter_signals;
pub(crate) use yahoo_rss::fetch_yahoo_news_rss;

use crate::types::{SentimentConfig, SentimentSignal};
use std::collections::HashSet;

/// Collect signals from all sources for a brand.
///
/// Continues past individual source failures, logging warnings.
/// Returns an empty `Vec` if all sources fail.
pub(crate) async fn collect_signals(
    config: &SentimentConfig,
    brand_slug: &str,
    brand_name: &str,
    brand_base_url: Option<&str>,
    twitter_handle: Option<&str>,
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
            tracing::warn!(
                brand = brand_slug,
                source = "google_news_rss",
                error = %e,
                "Google News RSS fetch failed"
            );
        }
    }

    // Bing News RSS
    match fetch_bing_news_rss(brand_slug, brand_name).await {
        Ok(signals_bing) => {
            tracing::debug!(
                brand = brand_slug,
                count = signals_bing.len(),
                "collected Bing RSS signals"
            );
            signals.extend(signals_bing);
        }
        Err(e) => {
            tracing::warn!(
                brand = brand_slug,
                source = "bing_news_rss",
                error = %e,
                "Bing RSS fetch failed"
            );
        }
    }

    // Yahoo News RSS
    match fetch_yahoo_news_rss(brand_slug, brand_name).await {
        Ok(signals_yahoo) => {
            tracing::debug!(
                brand = brand_slug,
                count = signals_yahoo.len(),
                "collected Yahoo RSS signals"
            );
            signals.extend(signals_yahoo);
        }
        Err(e) => {
            tracing::warn!(
                brand = brand_slug,
                source = "yahoo_news_rss",
                error = %e,
                "Yahoo RSS fetch failed"
            );
        }
    }

    // Brand newsroom / press pages
    let newsroom_signals =
        fetch_brand_newsroom_signals(brand_slug, brand_name, brand_base_url).await;
    tracing::debug!(
        brand = brand_slug,
        count = newsroom_signals.len(),
        "collected brand newsroom signals"
    );
    signals.extend(newsroom_signals);

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
                tracing::warn!(
                    brand = brand_slug,
                    source = "reddit",
                    error = %e,
                    "Reddit search failed"
                );
            }
        },
        Err(e) => {
            tracing::warn!(
                brand = brand_slug,
                source = "reddit",
                error = %e,
                "Reddit client creation failed"
            );
        }
    }

    // Twitter/X (bird CLI, optional creds)
    match fetch_twitter_signals(config, brand_slug, brand_name).await {
        Ok(twitter_signals) => {
            tracing::debug!(
                brand = brand_slug,
                count = twitter_signals.len(),
                "collected Twitter signals"
            );
            signals.extend(twitter_signals);
        }
        Err(e) => {
            tracing::warn!(
                brand = brand_slug,
                source = "twitter",
                error = %e,
                "Twitter source failed"
            );
        }
    }

    // Twitter/X brand timeline + replies (optional — only when handle is known)
    if let Some(handle) = twitter_handle {
        match fetch_twitter_brand_and_replies(config, brand_slug, handle).await {
            Ok(s) => {
                tracing::debug!(
                    brand = brand_slug,
                    count = s.len(),
                    "collected Twitter brand+reply signals"
                );
                signals.extend(s);
            }
            Err(e) => {
                tracing::warn!(
                    brand = brand_slug,
                    error = %e,
                    "Twitter brand timeline failed"
                );
            }
        }
    }

    // Dedup cross-source collisions by URL before embedding/scoring.
    let mut seen_urls: HashSet<String> = HashSet::new();
    signals.retain(|signal| seen_urls.insert(signal.url.clone()));

    signals
}
