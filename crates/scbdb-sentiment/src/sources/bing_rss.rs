//! Bing News RSS collector.

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use super::rss_helpers;
use crate::error::SentimentError;
use crate::types::SentimentSignal;

const MAX_SIGNALS: usize = 40;

pub(crate) async fn fetch_bing_news_rss(
    brand_slug: &str,
    brand_name: &str,
) -> Result<Vec<SentimentSignal>, SentimentError> {
    let query =
        format!("\"{brand_name}\" (hemp OR cbd OR thc OR cannabis) (beverage OR drink OR seltzer)");
    let encoded = utf8_percent_encode(&query, NON_ALPHANUMERIC).to_string();
    let url = format!("https://www.bing.com/news/search?q={encoded}&format=rss&mkt=en-US");

    let body = reqwest::get(url).await?.text().await?;
    rss_helpers::parse_rss_feed(&body, brand_slug, "bing_news", MAX_SIGNALS)
}
