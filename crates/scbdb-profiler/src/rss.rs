//! RSS/Atom feed crawler.
//!
//! Fetches an RSS or Atom feed URL and maps each entry to a [`CollectedSignal`].
//! Uses `feed-rs` for format-agnostic parsing (handles RSS 0.9x, RSS 1.0, RSS 2.0,
//! Atom 0.3/1.0, and JSON Feed).

use crate::{error::ProfilerError, types::CollectedSignal};
use reqwest::Client;
use tracing::debug;

/// Maximum summary length stored per signal (characters).
const MAX_SUMMARY_LEN: usize = 2000;

/// Crawl an RSS or Atom feed and return collected signals.
///
/// Returns one [`CollectedSignal`] per feed entry. Entries without a link are
/// still collected (the `source_url` field will be `None`).
///
/// # Errors
///
/// Returns [`ProfilerError::Http`] on network failure or
/// [`ProfilerError::Other`] on feed parsing errors.
pub async fn crawl_feed(
    client: &Client,
    brand_id: i64,
    feed_url: &str,
) -> Result<Vec<CollectedSignal>, ProfilerError> {
    debug!(feed_url, brand_id, "fetching feed");

    let response = client
        .get(feed_url)
        .header(
            reqwest::header::ACCEPT,
            "application/rss+xml, application/atom+xml, text/xml, application/xml",
        )
        .send()
        .await?
        .error_for_status()?;

    let bytes = response.bytes().await?;

    let feed = feed_rs::parser::parse(&bytes[..])
        .map_err(|e| ProfilerError::Other(format!("feed parse error: {e}")))?;

    let domain = extract_domain(feed_url);

    debug!(
        entry_count = feed.entries.len(),
        feed_url, "parsed feed entries"
    );

    let signals = feed
        .entries
        .into_iter()
        .map(|entry| {
            let summary = entry.summary.as_ref().map(|s| truncate(&s.content));

            CollectedSignal {
                brand_id,
                signal_type: "article".to_string(),
                source_platform: domain.clone(),
                source_url: entry.links.first().map(|l| l.href.clone()),
                external_id: Some(entry.id),
                title: entry.title.map(|t| t.content),
                summary,
                image_url: None,
                view_count: None,
                like_count: None,
                comment_count: None,
                share_count: None,
                published_at: entry.published,
            }
        })
        .collect();

    Ok(signals)
}

/// Extract the domain (host) from a URL for `source_platform`.
///
/// ```text
/// "https://blog.example.com/feed.xml" -> Some("blog.example.com")
/// "not-a-url"                         -> None
/// ```
fn extract_domain(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let domain = after_scheme.split('/').next()?;
    Some(domain.to_string())
}

/// Truncate a string to at most [`MAX_SUMMARY_LEN`] characters.
///
/// Operates on character boundaries so multi-byte content is never split
/// mid-codepoint.
fn truncate(s: &str) -> String {
    if s.chars().count() <= MAX_SUMMARY_LEN {
        s.to_string()
    } else {
        s.chars().take(MAX_SUMMARY_LEN).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- extract_domain -------------------------------------------------

    #[test]
    fn extract_domain_https() {
        assert_eq!(
            extract_domain("https://blog.example.com/feed.xml"),
            Some("blog.example.com".to_string()),
        );
    }

    #[test]
    fn extract_domain_no_path() {
        assert_eq!(
            extract_domain("https://example.com"),
            Some("example.com".to_string()),
        );
    }

    #[test]
    fn extract_domain_with_port() {
        assert_eq!(
            extract_domain("http://localhost:8080/rss"),
            Some("localhost:8080".to_string()),
        );
    }

    #[test]
    fn extract_domain_no_scheme() {
        assert_eq!(extract_domain("not-a-url"), None);
    }

    // -- truncate -------------------------------------------------------

    #[test]
    fn truncate_short_string() {
        let s = "hello";
        assert_eq!(truncate(s), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let long = "a".repeat(3000);
        let result = truncate(&long);
        assert_eq!(result.len(), MAX_SUMMARY_LEN);
    }

    #[test]
    fn truncate_exact_boundary() {
        let exact = "b".repeat(MAX_SUMMARY_LEN);
        assert_eq!(truncate(&exact), exact);
    }

    // -- feed parsing (offline) -----------------------------------------

    #[test]
    fn parse_rss_feed_entries() {
        let rss_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <item>
      <title>First Post</title>
      <link>https://example.com/post-1</link>
      <guid>post-1</guid>
      <description>Summary of the first post.</description>
      <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
    </item>
    <item>
      <title>Second Post</title>
      <link>https://example.com/post-2</link>
      <guid>post-2</guid>
      <description>Summary of the second post.</description>
    </item>
  </channel>
</rss>"#;

        let feed = feed_rs::parser::parse(&rss_xml[..]).expect("should parse valid RSS");

        let domain = extract_domain("https://example.com/feed.xml");

        let signals: Vec<CollectedSignal> = feed
            .entries
            .into_iter()
            .map(|entry| CollectedSignal {
                brand_id: 42,
                signal_type: "article".to_string(),
                source_platform: domain.clone(),
                source_url: entry.links.first().map(|l| l.href.clone()),
                external_id: Some(entry.id),
                title: entry.title.map(|t| t.content),
                summary: entry.summary.as_ref().map(|s| truncate(&s.content)),
                image_url: None,
                view_count: None,
                like_count: None,
                comment_count: None,
                share_count: None,
                published_at: entry.published,
            })
            .collect();

        assert_eq!(signals.len(), 2);
        assert_eq!(signals[0].signal_type, "article");
        assert_eq!(signals[0].source_platform.as_deref(), Some("example.com"),);
        assert_eq!(
            signals[0].source_url.as_deref(),
            Some("https://example.com/post-1"),
        );
        assert_eq!(signals[0].title.as_deref(), Some("First Post"));
        assert!(signals[0].summary.is_some());
        assert_eq!(signals[0].brand_id, 42);
        // Second entry has no pubDate
        assert!(signals[1].published_at.is_none());
    }

    #[test]
    fn parse_atom_feed_entries() {
        let atom_xml = br#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test Atom Feed</title>
  <entry>
    <title>Atom Entry</title>
    <id>urn:uuid:atom-entry-1</id>
    <link href="https://atom.example.com/entry-1"/>
    <summary>Summary of atom entry.</summary>
    <published>2024-06-15T10:00:00Z</published>
  </entry>
</feed>"#;

        let feed = feed_rs::parser::parse(&atom_xml[..]).expect("should parse valid Atom");

        assert_eq!(feed.entries.len(), 1);
        let entry = &feed.entries[0];
        assert_eq!(
            entry.title.as_ref().map(|t| t.content.as_str()),
            Some("Atom Entry")
        );
        assert!(entry.published.is_some());
        assert!(!entry.links.is_empty());
    }
}
