//! Reddit API signal collector (client-credentials OAuth).

use serde::Deserialize;

use crate::error::SentimentError;
use crate::types::{SentimentConfig, SentimentSignal};

/// Reddit OAuth token response.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Reddit search listing wrapper.
#[derive(Debug, Deserialize)]
struct Listing {
    data: ListingData,
}

#[derive(Debug, Deserialize)]
struct ListingData {
    children: Vec<Post>,
}

#[derive(Debug, Deserialize)]
struct Post {
    data: PostData,
}

#[derive(Debug, Deserialize)]
struct PostData {
    title: String,
    selftext: Option<String>,
    /// External URL from the Reddit post (part of API schema; we use `permalink` for dedup).
    #[allow(dead_code)]
    url: String,
    permalink: String,
}

/// Reddit API client with a valid access token.
pub(crate) struct RedditClient {
    client: reqwest::Client,
    token: String,
    user_agent: String,
}

impl RedditClient {
    /// Create a new `RedditClient` by exchanging client credentials for a token.
    ///
    /// # Errors
    ///
    /// Returns [`SentimentError::Reddit`] if token exchange fails.
    pub(crate) async fn new(config: &SentimentConfig) -> Result<Self, SentimentError> {
        // oauth.reddit.com uses TLS fingerprinting that blocks rustls. Use the
        // system OpenSSL stack so the handshake looks like standard curl/browser
        // traffic rather than a Rust-native TLS client.
        let client = reqwest::Client::builder()
            .use_native_tls()
            .build()
            .map_err(|e| SentimentError::Reddit(format!("failed to build HTTP client: {e}")))?;
        let token = Self::fetch_token(
            &client,
            &config.reddit_client_id,
            &config.reddit_client_secret,
            &config.reddit_user_agent,
        )
        .await?;

        Ok(Self {
            client,
            token,
            user_agent: config.reddit_user_agent.clone(),
        })
    }

    async fn fetch_token(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        user_agent: &str,
    ) -> Result<String, SentimentError> {
        let response = client
            .post("https://www.reddit.com/api/v1/access_token")
            .header("User-Agent", user_agent)
            .basic_auth(client_id, Some(client_secret))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SentimentError::Reddit(format!(
                "token exchange failed with status {}",
                response.status()
            )));
        }

        let token_resp: TokenResponse = response
            .json()
            .await
            .map_err(|e| SentimentError::Reddit(format!("token parse error: {e}")))?;

        Ok(token_resp.access_token)
    }

    /// Search Reddit for brand mentions across hemp/cannabis subreddits.
    ///
    /// Returns up to 25 signals per brand (one page, no pagination for MVP).
    ///
    /// # Errors
    ///
    /// Returns [`SentimentError::Reddit`] if the search request fails.
    pub(crate) async fn search_brand_mentions(
        &self,
        brand_slug: &str,
        brand_name: &str,
    ) -> Result<Vec<SentimentSignal>, SentimentError> {
        let subreddits = "delta8+hemp+cannabis+delta_8+hempflowers+CBD";
        let url = format!(
            "https://oauth.reddit.com/r/{subreddits}/search?q={brand_name}&restrict_sr=true&limit=25&sort=new"
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SentimentError::Reddit(format!(
                "Reddit search failed with status {}",
                response.status()
            )));
        }

        let listing: Listing = response
            .json()
            .await
            .map_err(|e| SentimentError::Reddit(format!("Reddit response parse error: {e}")))?;

        let signals = listing
            .data
            .children
            .into_iter()
            .map(|post| {
                let text = match post.data.selftext.as_deref() {
                    Some(body) if !body.is_empty() && body != "[deleted]" => {
                        let snippet: String = body.chars().take(280).collect();
                        format!("{} {}", post.data.title, snippet)
                    }
                    _ => post.data.title.clone(),
                };
                SentimentSignal {
                    text,
                    url: format!("https://reddit.com{}", post.data.permalink),
                    source: "reddit".to_string(),
                    brand_slug: brand_slug.to_string(),
                    score: 0.0,
                }
            })
            .collect();

        Ok(signals)
    }
}
