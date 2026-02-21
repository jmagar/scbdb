//! Reddit API signal collector (client-credentials OAuth).

use std::collections::HashSet;

use serde::Deserialize;

use crate::error::SentimentError;
use crate::types::{SentimentConfig, SentimentSignal};

const SEARCH_SUBREDDITS: &str =
    "delta8+hemp+cannabis+delta_8+hempflowers+CBD+weed+treedibles+altcannabinoids+cannabisextracts";
const PAGE_LIMIT: usize = 50;
const PAGE_COUNT: usize = 2;
const REDDIT_MAX_SIGNALS: usize = 60;

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
    after: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Post {
    data: PostData,
}

#[derive(Debug, Deserialize)]
struct PostData {
    title: Option<String>,
    selftext: Option<String>,
    body: Option<String>,
    permalink: Option<String>,
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
    /// Pulls posts with multiple query variants and fallback global search.
    ///
    /// # Errors
    ///
    /// Returns [`SentimentError::Reddit`] if the search request fails.
    pub(crate) async fn search_brand_mentions(
        &self,
        brand_slug: &str,
        brand_name: &str,
    ) -> Result<Vec<SentimentSignal>, SentimentError> {
        let mut signals = Vec::new();
        let mut seen_urls = HashSet::new();
        let brand_terms = build_brand_terms(brand_slug, brand_name);

        let queries = build_query_variants(brand_slug, brand_name);
        let kinds = ["link"];
        let sorts = ["relevance", "new"];

        'search: for query in &queries {
            for kind in &kinds {
                for sort in &sorts {
                    let page = self
                        .search_kind(brand_slug, query, kind, sort, true)
                        .await?;

                    for signal in page {
                        if !mentions_brand(&signal.text, &brand_terms) {
                            continue;
                        }
                        if seen_urls.insert(signal.url.clone()) {
                            signals.push(signal);
                        }
                        if signals.len() >= REDDIT_MAX_SIGNALS {
                            break 'search;
                        }
                    }
                }
            }
        }

        if signals.len() < 10 {
            for sort in &sorts {
                let page = self
                    .search_kind(brand_slug, brand_name, "link", sort, false)
                    .await?;
                for signal in page {
                    if !mentions_brand(&signal.text, &brand_terms) {
                        continue;
                    }
                    if seen_urls.insert(signal.url.clone()) {
                        signals.push(signal);
                    }
                    if signals.len() >= REDDIT_MAX_SIGNALS {
                        break;
                    }
                }
                if signals.len() >= REDDIT_MAX_SIGNALS {
                    break;
                }
            }
        }

        tracing::debug!(
            brand = brand_slug,
            query_variants = queries.len(),
            signals = signals.len(),
            "collected Reddit signals"
        );

        Ok(signals)
    }

    async fn search_kind(
        &self,
        brand_slug: &str,
        query: &str,
        kind: &str,
        sort: &str,
        restrict_subreddits: bool,
    ) -> Result<Vec<SentimentSignal>, SentimentError> {
        let mut after: Option<String> = None;
        let mut all_signals = Vec::new();

        for _ in 0..PAGE_COUNT {
            let mut params: Vec<(&str, String)> = vec![
                ("q", query.to_string()),
                (
                    "restrict_sr",
                    if restrict_subreddits { "true" } else { "false" }.to_string(),
                ),
                ("sort", sort.to_string()),
                ("limit", PAGE_LIMIT.to_string()),
                ("type", kind.to_string()),
            ];
            if let Some(cursor) = &after {
                params.push(("after", cursor.clone()));
            }

            let endpoint = if restrict_subreddits {
                format!("https://oauth.reddit.com/r/{SEARCH_SUBREDDITS}/search")
            } else {
                "https://oauth.reddit.com/search".to_string()
            };

            let req = self
                .client
                .get(endpoint)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("User-Agent", &self.user_agent)
                .query(&params);

            let response = req.send().await?;

            if !response.status().is_success() {
                return Err(SentimentError::Reddit(format!(
                    "Reddit search failed ({kind}) with status {}",
                    response.status()
                )));
            }

            let listing: Listing = response
                .json()
                .await
                .map_err(|e| SentimentError::Reddit(format!("Reddit response parse error: {e}")))?;

            let page_signals: Vec<SentimentSignal> = listing
                .data
                .children
                .iter()
                .filter_map(|post| to_signal(post, brand_slug, kind))
                .collect();

            all_signals.extend(page_signals);
            if all_signals.len() >= REDDIT_MAX_SIGNALS {
                break;
            }

            after = listing.data.after;
            if after.is_none() {
                break;
            }
        }

        Ok(all_signals)
    }
}

fn build_query_variants(brand_slug: &str, brand_name: &str) -> Vec<String> {
    let slug_query = brand_slug.replace('-', " ");
    let mut variants = vec![
        format!("\"{brand_name}\""),
        brand_name.to_string(),
        slug_query,
        format!("\"{brand_name}\" (thc OR cbd OR hemp OR cannabis)"),
        format!("\"{brand_name}\" (drink OR beverage OR seltzer)"),
    ];

    // Collapse duplicates while preserving order.
    let mut seen = HashSet::new();
    variants.retain(|q| seen.insert(q.to_lowercase()));
    variants
}

fn build_brand_terms(brand_slug: &str, brand_name: &str) -> Vec<String> {
    let mut terms = vec![
        normalize_text_for_match(brand_name),
        normalize_text_for_match(&brand_name.replace('\'', "")),
        normalize_text_for_match(&brand_slug.replace('-', " ")),
    ];
    terms.retain(|t| !t.is_empty());
    terms.sort();
    terms.dedup();
    terms
}

fn mentions_brand(text: &str, brand_terms: &[String]) -> bool {
    let normalized = normalize_text_for_match(text);
    let padded = format!(" {normalized} ");
    let compact = normalized.replace(' ', "");
    brand_terms.iter().any(|term| {
        if term.len() < 3 {
            return false;
        }
        let needle = format!(" {term} ");
        if padded.contains(&needle) {
            return true;
        }
        let compact_term = term.replace(' ', "");
        compact_term.len() >= 6 && compact.contains(&compact_term)
    })
}

fn normalize_text_for_match(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn to_signal(post: &Post, brand_slug: &str, kind: &str) -> Option<SentimentSignal> {
    let permalink = post.data.permalink.as_ref()?;
    let url = format!("https://reddit.com{permalink}");

    let text = if kind == "comment" {
        post.data
            .body
            .as_deref()
            .map(str::trim)
            .filter(|body| !body.is_empty() && *body != "[deleted]" && *body != "[removed]")
            .map(|body| body.chars().take(420).collect::<String>())?
    } else {
        let title = post
            .data
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())?;

        let text = match post.data.selftext.as_deref() {
            Some(body) if !body.is_empty() && body != "[deleted]" && body != "[removed]" => {
                let snippet: String = body.chars().take(280).collect();
                format!("{title} {snippet}")
            }
            _ => title.to_string(),
        };

        text
    };

    Some(SentimentSignal {
        text,
        url,
        source: if kind == "comment" {
            "reddit_comment".to_string()
        } else {
            "reddit_post".to_string()
        },
        brand_slug: brand_slug.to_string(),
        score: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_query_variants, mentions_brand};

    #[test]
    fn query_variants_include_brand_and_slug_forms() {
        let variants = build_query_variants("uncle-arnies", "Uncle Arnie's");
        assert!(variants.iter().any(|q| q.contains("\"Uncle Arnie's\"")));
        assert!(variants.iter().any(|q| q.contains("uncle arnies")));
        assert!(
            variants.iter().any(|q| q.contains("thc OR cbd")),
            "expected keyword-expanded query variant"
        );
    }

    #[test]
    fn mention_filter_matches_whole_phrase_and_rejects_near_miss() {
        let terms = vec!["uncle arnies".to_string()];
        assert!(mentions_brand("I love Uncle Arnie's iced tea", &terms));
        assert!(!mentions_brand("I love my uncle's iced tea", &terms));
    }
}
