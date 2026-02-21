//! `YouTube` search RSS signal collector.

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::SentimentError;
use crate::types::SentimentSignal;

const MAX_SIGNALS: usize = 30;

/// Fetch recent `YouTube` search feed entries for a brand.
///
/// # Errors
///
/// Returns [`SentimentError::Http`] for network failures and
/// [`SentimentError::Xml`] for malformed feed content.
pub(crate) async fn fetch_youtube_search_rss(
    brand_slug: &str,
    brand_name: &str,
) -> Result<Vec<SentimentSignal>, SentimentError> {
    let query = format!("{brand_name} hemp cbd thc drink");
    let encoded = utf8_percent_encode(&query, NON_ALPHANUMERIC).to_string();
    let url = format!("https://www.youtube.com/feeds/videos.xml?search_query={encoded}");

    let body = reqwest::get(url).await?.text().await?;
    parse_youtube_feed(&body, brand_slug)
}

fn parse_youtube_feed(xml: &str, brand_slug: &str) -> Result<Vec<SentimentSignal>, SentimentError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut in_entry = false;
    let mut current_tag = String::new();
    let mut title = String::new();
    let mut video_url = String::new();
    let mut signals = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name_buf = e.name().as_ref().to_vec();
                let name = std::str::from_utf8(&name_buf).unwrap_or("");
                match name {
                    "entry" => {
                        in_entry = true;
                        title.clear();
                        video_url.clear();
                    }
                    "link" if in_entry => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"href" {
                                let href =
                                    String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                if href.contains("youtube.com/watch") {
                                    video_url = href;
                                }
                            }
                        }
                    }
                    _ => {
                        current_tag = name.to_string();
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                if in_entry {
                    let name_buf = e.name().as_ref().to_vec();
                    let name = std::str::from_utf8(&name_buf).unwrap_or("");
                    if name == "link" {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"href" {
                                let href = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                if href.contains("youtube.com/watch") {
                                    video_url = href;
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_entry && current_tag == "title" {
                    title = e.unescape().unwrap_or_default().into_owned();
                }
            }
            Ok(Event::End(e)) => {
                let name_buf = e.name().as_ref().to_vec();
                let name = std::str::from_utf8(&name_buf).unwrap_or("");
                if name == "entry" {
                    in_entry = false;
                    if !title.is_empty() && !video_url.is_empty() {
                        signals.push(SentimentSignal {
                            text: title.clone(),
                            url: video_url.clone(),
                            source: "youtube_rss".to_string(),
                            brand_slug: brand_slug.to_string(),
                            score: 0.0,
                        });
                        if signals.len() >= MAX_SIGNALS {
                            break;
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(SentimentError::Xml(e)),
            _ => {}
        }
    }

    Ok(signals)
}

#[cfg(test)]
mod tests {
    use super::parse_youtube_feed;

    #[test]
    fn parses_feed_entries_into_signals() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title>Cann review</title>
    <link rel="alternate" href="https://www.youtube.com/watch?v=abc123"/>
  </entry>
</feed>"#;
        let out = parse_youtube_feed(xml, "cann").expect("parse");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].source, "youtube_rss");
    }
}
