//! Twitter/X signal source via the `bird` CLI.
//!
//! Invokes `bird search "{query}" --json -n 50 --auth-token ... --ct0 ...`
//! as a subprocess. Returns an empty vec if credentials are not configured.

use std::collections::HashSet;

use serde::Deserialize;

use crate::{
    error::SentimentError,
    types::{SentimentConfig, SentimentSignal},
};

#[derive(Deserialize)]
struct BirdTweet {
    id: String,
    text: String,
    author: BirdAuthor,
}

#[derive(Deserialize)]
struct BirdAuthor {
    username: String,
}

/// Fetch Twitter signals for a brand using the `bird` CLI.
///
/// Silently returns an empty `Vec` when Twitter credentials are absent.
///
/// # Errors
///
/// Returns `SentimentError::Twitter` if the `bird` subprocess cannot be
/// spawned. Non-zero exits from `bird` are treated as warnings and skipped.
pub(crate) async fn fetch_twitter_signals(
    config: &SentimentConfig,
    brand_slug: &str,
    brand_name: &str,
) -> Result<Vec<SentimentSignal>, SentimentError> {
    let (auth_token, ct0) = match (&config.twitter_auth_token, &config.twitter_ct0) {
        (Some(a), Some(c)) => (a.as_str(), c.as_str()),
        _ => return Ok(vec![]),
    };

    let queries = [
        format!(r#""{brand_name}" (hemp OR thc OR cbd OR beverage OR drink OR seltzer)"#),
        format!(r#""{brand_name}" cannabis"#),
    ];

    let mut signals = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for query in &queries {
        let output = tokio::process::Command::new("bird")
            .args([
                "search",
                query,
                "--json",
                "-n",
                "50",
                "--auth-token",
                auth_token,
                "--ct0",
                ct0,
            ])
            .output()
            .await
            .map_err(|e| SentimentError::Twitter(format!("bird subprocess error: {e}")))?;

        if !output.status.success() {
            tracing::warn!(
                brand = brand_slug,
                query = query.as_str(),
                "bird search returned non-zero exit"
            );
            continue;
        }

        let tweets: Vec<BirdTweet> = serde_json::from_slice(&output.stdout).unwrap_or_default();

        for tweet in tweets {
            let url = format!(
                "https://x.com/{}/status/{}",
                tweet.author.username, tweet.id
            );
            if !seen.insert(url.clone()) {
                continue;
            }
            signals.push(SentimentSignal {
                text: tweet.text,
                url,
                source: "twitter".to_string(),
                brand_slug: brand_slug.to_string(),
                score: 0.0,
            });
        }
    }

    Ok(signals)
}

/// Fetch brand's own recent tweets and replies to those tweets.
///
/// Returns two source types:
/// - `twitter_brand` — posts from the brand's own timeline
/// - `twitter_replies` — replies received on those posts
///
/// Silently returns an empty `Vec` when Twitter credentials are absent.
///
/// # Errors
///
/// Returns `SentimentError::Twitter` if the `bird` subprocess cannot be spawned.
pub(crate) async fn fetch_twitter_brand_and_replies(
    config: &SentimentConfig,
    brand_slug: &str,
    handle: &str,
) -> Result<Vec<SentimentSignal>, SentimentError> {
    let (auth_token, ct0) = match (&config.twitter_auth_token, &config.twitter_ct0) {
        (Some(a), Some(c)) => (a.as_str(), c.as_str()),
        _ => return Ok(vec![]),
    };

    // Fetch brand's own 20 most recent tweets
    let output = tokio::process::Command::new("bird")
        .args([
            "user-tweets",
            handle,
            "--json",
            "-n",
            "20",
            "--auth-token",
            auth_token,
            "--ct0",
            ct0,
        ])
        .output()
        .await
        .map_err(|e| SentimentError::Twitter(format!("bird user-tweets: {e}")))?;

    if !output.status.success() {
        tracing::warn!(brand = brand_slug, handle, "bird user-tweets non-zero exit");
        return Ok(vec![]);
    }

    let brand_tweets: Vec<BirdTweet> = serde_json::from_slice(&output.stdout).unwrap_or_default();
    let mut signals = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for tweet in &brand_tweets {
        let url = format!("https://x.com/{handle}/status/{}", tweet.id);
        if seen.insert(url.clone()) {
            signals.push(SentimentSignal {
                text: tweet.text.clone(),
                url,
                source: "twitter_brand".to_string(),
                brand_slug: brand_slug.to_string(),
                score: 0.0,
            });
        }
    }

    // Fetch replies for the 10 most recent brand tweets, capped at 20 replies each
    for tweet in brand_tweets.iter().take(10) {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let output = tokio::process::Command::new("bird")
            .args([
                "replies",
                &tweet.id,
                "--json",
                "-n",
                "20",
                "--auth-token",
                auth_token,
                "--ct0",
                ct0,
            ])
            .output()
            .await
            .map_err(|e| SentimentError::Twitter(format!("bird replies: {e}")))?;

        if !output.status.success() {
            tracing::warn!(
                brand = brand_slug,
                tweet_id = %tweet.id,
                "bird replies non-zero exit"
            );
            continue;
        }

        let replies: Vec<BirdTweet> = serde_json::from_slice(&output.stdout).unwrap_or_default();
        for reply in replies {
            let url = format!(
                "https://x.com/{}/status/{}",
                reply.author.username, reply.id
            );
            if seen.insert(url.clone()) {
                signals.push(SentimentSignal {
                    text: reply.text,
                    url,
                    source: "twitter_replies".to_string(),
                    brand_slug: brand_slug.to_string(),
                    score: 0.0,
                });
            }
        }
    }

    Ok(signals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_bird_tweet() {
        let json = r#"[
            {
                "id": "1234567890",
                "text": "Trying this hemp seltzer — surprisingly good",
                "author": { "username": "testuser" }
            }
        ]"#;
        let tweets: Vec<BirdTweet> = serde_json::from_str(json).unwrap();
        assert_eq!(tweets.len(), 1);
        assert_eq!(tweets[0].id, "1234567890");
        assert_eq!(tweets[0].author.username, "testuser");
    }

    /// Live integration test: requires `TWITTER_AUTH_TOKEN` + `TWITTER_CT0` in env.
    /// Run with: `cargo test -p scbdb-sentiment twitter_live -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn twitter_live_fetch() {
        let auth_token = std::env::var("TWITTER_AUTH_TOKEN").expect("TWITTER_AUTH_TOKEN not set");
        let ct0 = std::env::var("TWITTER_CT0").expect("TWITTER_CT0 not set");
        let config = crate::types::SentimentConfig {
            tei_url: String::new(),
            qdrant_url: String::new(),
            qdrant_collection: String::new(),
            reddit_client_id: String::new(),
            reddit_client_secret: String::new(),
            reddit_user_agent: String::new(),
            twitter_auth_token: Some(auth_token),
            twitter_ct0: Some(ct0),
        };
        let signals = fetch_twitter_signals(&config, "cann", "Cann")
            .await
            .expect("fetch should succeed");
        assert!(
            !signals.is_empty(),
            "expected at least one Twitter signal for 'Cann'"
        );
        assert!(signals.iter().all(|s| s.source == "twitter"));
        assert!(signals.iter().all(|s| s.url.starts_with("https://x.com/")));
        println!("got {} Twitter signals for Cann", signals.len());
        println!(
            "first: {} — {}",
            signals[0].url,
            &signals[0].text[..60.min(signals[0].text.len())]
        );
    }

    #[test]
    fn deserialize_extra_fields_ignored() {
        let json = r#"[
            {
                "id": "999",
                "text": "hemp beverage",
                "createdAt": "2024-01-01T00:00:00Z",
                "likeCount": 42,
                "retweetCount": 7,
                "author": { "username": "hemp_fan", "name": "Hemp Fan" },
                "authorId": "111"
            }
        ]"#;
        let tweets: Vec<BirdTweet> = serde_json::from_str(json).unwrap();
        assert_eq!(tweets[0].author.username, "hemp_fan");
    }

    #[test]
    fn brand_and_replies_returns_empty_without_creds() {
        let config = SentimentConfig {
            tei_url: String::new(),
            qdrant_url: String::new(),
            qdrant_collection: String::new(),
            reddit_client_id: String::new(),
            reddit_client_secret: String::new(),
            reddit_user_agent: String::new(),
            twitter_auth_token: None,
            twitter_ct0: None,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(fetch_twitter_brand_and_replies(
            &config,
            "cann",
            "drinkcann",
        ));
        assert!(result.unwrap().is_empty());
    }

    /// Live integration test: requires `TWITTER_AUTH_TOKEN` + `TWITTER_CT0` in env.
    /// Run with: `cargo test -p scbdb-sentiment brand_timeline_live -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn brand_timeline_live_cann() {
        let auth_token = std::env::var("TWITTER_AUTH_TOKEN").expect("TWITTER_AUTH_TOKEN not set");
        let ct0 = std::env::var("TWITTER_CT0").expect("TWITTER_CT0 not set");
        let config = SentimentConfig {
            tei_url: String::new(),
            qdrant_url: String::new(),
            qdrant_collection: String::new(),
            reddit_client_id: String::new(),
            reddit_client_secret: String::new(),
            reddit_user_agent: String::new(),
            twitter_auth_token: Some(auth_token),
            twitter_ct0: Some(ct0),
        };
        let signals = fetch_twitter_brand_and_replies(&config, "cann", "drinkcann")
            .await
            .unwrap();
        assert!(!signals.is_empty());
        let brand_signals: Vec<_> = signals
            .iter()
            .filter(|s| s.source == "twitter_brand")
            .collect();
        let reply_signals: Vec<_> = signals
            .iter()
            .filter(|s| s.source == "twitter_replies")
            .collect();
        assert!(!brand_signals.is_empty());
        assert!(brand_signals
            .iter()
            .all(|s| s.url.contains("x.com/drinkcann/status/")));
        assert!(reply_signals
            .iter()
            .all(|s| s.url.starts_with("https://x.com/")));
    }
}
