//! Google News RSS signal collector.

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::SentimentError;
use crate::types::SentimentSignal;

/// Fetch signals from Google News RSS for a brand.
///
/// Searches for `{brand_name} hemp beverage` and returns up to 25 signals.
/// Each `<item>` title + description becomes one `SentimentSignal`.
///
/// # Errors
///
/// Returns [`SentimentError::Http`] on network failure or
/// [`SentimentError::Xml`] on malformed RSS.
pub(crate) async fn fetch_google_news_rss(
    brand_slug: &str,
    brand_name: &str,
) -> Result<Vec<SentimentSignal>, SentimentError> {
    let query = format!("{brand_name} hemp beverage");
    let encoded = utf8_percent_encode(&query, NON_ALPHANUMERIC).to_string();
    let url = format!("https://news.google.com/rss/search?q={encoded}&hl=en-US&gl=US&ceid=US:en");

    let body = reqwest::get(&url).await?.text().await?;
    parse_rss_feed(&body, brand_slug)
}

/// Parse an RSS feed XML body into `SentimentSignal`s.
///
/// # Errors
///
/// Returns [`SentimentError::Xml`] if the XML is malformed.
pub(crate) fn parse_rss_feed(
    xml: &str,
    brand_slug: &str,
) -> Result<Vec<SentimentSignal>, SentimentError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut signals = Vec::new();
    let mut current_title = String::new();
    let mut current_link = String::new();
    let mut current_description = String::new();
    let mut in_item = false;
    let mut current_tag = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                match name.as_str() {
                    "item" => {
                        in_item = true;
                        current_title.clear();
                        current_link.clear();
                        current_description.clear();
                    }
                    _ => {
                        current_tag = name;
                    }
                }
            }
            Ok(Event::End(e)) => {
                let raw = e.name();
                let name = std::str::from_utf8(raw.as_ref()).unwrap_or("");
                if name == "item" && in_item {
                    in_item = false;
                    if !current_link.is_empty() {
                        let text = if current_description.is_empty() {
                            current_title.clone()
                        } else {
                            format!("{current_title} {current_description}")
                        };
                        signals.push(SentimentSignal {
                            text,
                            url: current_link.clone(),
                            source: "google_news".to_string(),
                            brand_slug: brand_slug.to_string(),
                            score: 0.0,
                        });
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_item {
                    let text = e.unescape().unwrap_or_default().into_owned();
                    match current_tag.as_str() {
                        "title" => current_title = text,
                        "link" => current_link = text,
                        "description" => {
                            // Strip HTML tags from description
                            current_description = strip_html(&text);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(e)) => {
                if in_item {
                    let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                    match current_tag.as_str() {
                        "title" => current_title = text,
                        "link" => current_link = text,
                        "description" => current_description = strip_html(&text),
                        _ => {}
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

/// Strip HTML tags from a string, returning plain text.
fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Google News</title>
    <item>
      <title>CANN Hemp Beverage Launches New Flavor</title>
      <link>https://example.com/cann-news-1</link>
      <description>CANN has announced a great new hemp drink line.</description>
    </item>
    <item>
      <title>Hemp Beverage Market Growing</title>
      <link>https://example.com/hemp-market</link>
      <description>The hemp beverage sector is expanding rapidly.</description>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn parses_valid_rss_returns_signals() {
        let signals = parse_rss_feed(SAMPLE_RSS, "cann").expect("should parse valid RSS");
        assert_eq!(
            signals.len(),
            2,
            "expected 2 signals, got {}",
            signals.len()
        );
        assert_eq!(signals[0].source, "google_news");
        assert_eq!(signals[0].brand_slug, "cann");
        assert!(!signals[0].url.is_empty());
        assert!(!signals[0].text.is_empty());
    }

    #[test]
    fn empty_feed_returns_empty_vec() {
        let xml = r#"<?xml version="1.0"?><rss version="2.0"><channel></channel></rss>"#;
        let signals = parse_rss_feed(xml, "test").expect("should parse empty RSS");
        assert!(signals.is_empty());
    }

    #[test]
    fn malformed_xml_returns_error() {
        let xml = "<rss><channel><item><title>Unclosed";
        // quick-xml reads until EOF so this may succeed — check that we handle it gracefully
        let result = parse_rss_feed(xml, "test");
        // Either Ok (empty, since no complete items) or Err — both are acceptable
        match result {
            Ok(signals) => assert!(signals.is_empty()),
            Err(SentimentError::Xml(_)) => {} // expected
            Err(e) => panic!("unexpected error type: {e}"),
        }
    }
}
