//! Shared RSS/XML feed parsing and HTML stripping helpers.
//!
//! Used by [`super::bing_rss`] and [`super::yahoo_rss`] to avoid duplicating
//! the item-extraction and HTML-stripping logic.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::SentimentError;
use crate::types::SentimentSignal;

/// Parse an RSS XML feed into [`SentimentSignal`]s.
///
/// Extracts `<item>` elements, pulling `<title>`, `<link>`, and `<description>`
/// fields. HTML tags in descriptions are stripped. Stops after `max_signals`
/// items have been collected.
pub(crate) fn parse_rss_feed(
    xml: &str,
    brand_slug: &str,
    source: &str,
    max_signals: usize,
) -> Result<Vec<SentimentSignal>, SentimentError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut signals = Vec::new();
    let mut in_item = false;
    let mut in_description = false;
    let mut current_tag = String::new();
    let mut title = String::new();
    let mut link = String::new();
    let mut description = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name_buf = e.name().as_ref().to_vec();
                let name = std::str::from_utf8(&name_buf).unwrap_or("").to_string();
                if name == "item" {
                    in_item = true;
                    in_description = false;
                    title.clear();
                    link.clear();
                    description.clear();
                } else if name == "description" && in_item {
                    in_description = true;
                }
                current_tag = name;
            }
            Ok(Event::End(e)) => {
                let name_buf = e.name().as_ref().to_vec();
                let name = std::str::from_utf8(&name_buf).unwrap_or("");
                if name == "description" {
                    in_description = false;
                }
                if name == "item" && in_item {
                    in_item = false;
                    if !title.is_empty() && !link.is_empty() {
                        let text = if description.is_empty() {
                            title.clone()
                        } else {
                            format!("{title} {description}")
                        };
                        signals.push(SentimentSignal {
                            text,
                            url: link.clone(),
                            source: source.to_string(),
                            brand_slug: brand_slug.to_string(),
                            score: 0.0,
                        });
                        if signals.len() >= max_signals {
                            break;
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_item {
                    let text = e.unescape().unwrap_or_default().into_owned();
                    if in_description {
                        // Accumulate all text nodes inside <description>,
                        // including those emitted after nested tags like <b>.
                        if !description.is_empty() {
                            description.push(' ');
                        }
                        description.push_str(&text);
                    } else {
                        match current_tag.as_str() {
                            "title" => title = text,
                            "link" => link = text,
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::CData(e)) => {
                if in_item && in_description {
                    let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                    description = strip_html(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(SentimentError::Xml(e)),
            _ => {}
        }
    }

    Ok(signals)
}

/// Strip HTML tags from a string and normalize whitespace.
pub(crate) fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
