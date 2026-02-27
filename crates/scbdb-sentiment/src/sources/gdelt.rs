//! GDELT document API signal collector.

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;

use crate::error::SentimentError;
use crate::types::SentimentSignal;

const MAX_SIGNALS: usize = 40;

#[derive(Debug, Deserialize)]
struct GdeltResponse {
    #[serde(default)]
    articles: Vec<GdeltArticle>,
}

#[derive(Debug, Deserialize)]
struct GdeltArticle {
    url: Option<String>,
    title: Option<String>,
    #[serde(rename = "seendate")]
    _seen_date: Option<String>,
}

/// Fetchs recent news mentions from GDELT Doc API.
///
/// # Errors
///
/// Returns [`SentimentError::Http`] on request/parse failures.
pub(crate) async fn fetch_gdelt_news(
    brand_slug: &str,
    brand_name: &str,
) -> Result<Vec<SentimentSignal>, SentimentError> {
    let query = format!(
        "\"{brand_name}\" AND (hemp OR cbd OR thc OR cannabis) AND (drink OR beverage OR seltzer)"
    );
    let encoded = utf8_percent_encode(&query, NON_ALPHANUMERIC).to_string();
    let url = format!(
        "https://api.gdeltproject.org/api/v2/doc/doc?query={encoded}&mode=ArtList&maxrecords={MAX_SIGNALS}&sort=DateDesc&format=json"
    );

    let response: GdeltResponse = reqwest::get(url).await?.json().await?;

    let signals = response
        .articles
        .into_iter()
        .filter_map(|article| {
            let url = article.url?;
            let title = article.title.unwrap_or_default();
            if title.trim().is_empty() {
                return None;
            }
            Some(SentimentSignal {
                text: title,
                url,
                source: "gdelt_news".to_string(),
                brand_slug: brand_slug.to_string(),
                score: 0.0,
            })
        })
        .take(MAX_SIGNALS)
        .collect();

    Ok(signals)
}
