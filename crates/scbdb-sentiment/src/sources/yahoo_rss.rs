//! Yahoo News RSS collector.

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::error::SentimentError;
use crate::types::SentimentSignal;

const MAX_SIGNALS: usize = 40;

pub(crate) async fn fetch_yahoo_news_rss(
    brand_slug: &str,
    brand_name: &str,
) -> Result<Vec<SentimentSignal>, SentimentError> {
    let query =
        format!("\"{brand_name}\" (hemp OR cbd OR thc OR cannabis) (beverage OR drink OR seltzer)");
    let encoded = utf8_percent_encode(&query, NON_ALPHANUMERIC).to_string();
    let url = format!("https://news.search.yahoo.com/rss?p={encoded}");

    let body = reqwest::get(url).await?.text().await?;
    if !body.contains("<rss") && !body.contains("<feed") {
        return Ok(Vec::new());
    }
    parse_feed(&body, brand_slug)
}

fn parse_feed(xml: &str, brand_slug: &str) -> Result<Vec<SentimentSignal>, SentimentError> {
    // Reuse Bing parser logic style to avoid pulling additional crates.
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut signals = Vec::new();
    let mut in_item = false;
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
                    title.clear();
                    link.clear();
                    description.clear();
                }
                current_tag = name;
            }
            Ok(Event::End(e)) => {
                let name_buf = e.name().as_ref().to_vec();
                let name = std::str::from_utf8(&name_buf).unwrap_or("");
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
                            source: "yahoo_news".to_string(),
                            brand_slug: brand_slug.to_string(),
                            score: 0.0,
                        });
                        if signals.len() >= MAX_SIGNALS {
                            break;
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_item {
                    let text = e.unescape().unwrap_or_default().into_owned();
                    match current_tag.as_str() {
                        "title" => title = text,
                        "link" => link = text,
                        "description" => description = strip_html(&text),
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(e)) => {
                if in_item {
                    let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                    if current_tag == "description" {
                        description = strip_html(&text);
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

fn strip_html(html: &str) -> String {
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
