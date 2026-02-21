//! HTML parsing helpers for the `VTInfo` store locator response.

use regex::Regex;

use crate::locator::types::RawStoreLocation;

pub(super) fn parse_vtinfo_search_results(html: &str) -> Vec<RawStoreLocation> {
    let article_re =
        Regex::new(r"(?s)<article[^>]*finder_location[^>]*>.*?</article>").expect("valid regex");
    article_re
        .find_iter(html)
        .filter_map(|m| parse_location_article(m.as_str()))
        .collect()
}

fn parse_location_article(article: &str) -> Option<RawStoreLocation> {
    let name = strip_html(
        Regex::new(r"(?s)<h2[^>]*finder_dba_text[^>]*>(.*?)</h2>")
            .expect("valid regex")
            .captures(article)?
            .get(1)?
            .as_str(),
    );
    if name.is_empty() {
        return None;
    }

    let lat = Regex::new(r#"data-latitude=\"([^\"]+)\""#)
        .expect("valid regex")
        .captures(article)
        .and_then(|cap| cap.get(1).and_then(|m| m.as_str().parse::<f64>().ok()));
    let lng = Regex::new(r#"data-longitude=\"([^\"]+)\""#)
        .expect("valid regex")
        .captures(article)
        .and_then(|cap| cap.get(1).and_then(|m| m.as_str().parse::<f64>().ok()));

    let address_line1 =
        Regex::new(r#"(?s)<a[^>]*class=\"finder_address\"[^>]*>\s*<span>(.*?)</span>"#)
            .expect("valid regex")
            .captures(article)
            .and_then(|cap| cap.get(1).map(|m| strip_html(m.as_str())))
            .filter(|s| !s.is_empty());

    let city = Regex::new(r#"<span class=\"finder_address_city\">(.*?)</span>"#)
        .expect("valid regex")
        .captures(article)
        .and_then(|cap| cap.get(1).map(|m| decode_html(m.as_str())))
        .filter(|s| !s.trim().is_empty());

    let state = Regex::new(r#"<span class=\"finder_address_state\">(.*?)</span>"#)
        .expect("valid regex")
        .captures(article)
        .and_then(|cap| cap.get(1).map(|m| decode_html(m.as_str())))
        .filter(|s| !s.trim().is_empty());

    let phone = Regex::new(r#"(?s)<a[^>]*href=\"tel:[^\"]+\"[^>]*>\s*<span>(.*?)</span>"#)
        .expect("valid regex")
        .captures(article)
        .and_then(|cap| cap.get(1).map(|m| strip_html(m.as_str())))
        .filter(|s| !s.is_empty());

    Some(RawStoreLocation {
        external_id: None,
        name,
        address_line1,
        city,
        state,
        zip: None,
        country: Some("US".to_string()),
        latitude: lat,
        longitude: lng,
        phone,
        locator_source: "vtinfo".to_string(),
        raw_data: serde_json::json!({"html": article}),
    })
}

pub(super) fn extract_hidden_input_value(html: &str, name: &str) -> Option<String> {
    let pattern = format!(
        r#"<input[^>]*name=\"{}\"[^>]*value=\"([^\"]*)\""#,
        regex::escape(name)
    );
    Regex::new(&pattern)
        .ok()?
        .captures(html)
        .and_then(|cap| cap.get(1).map(|m| decode_html(m.as_str())))
}

pub(super) fn extract_js_string_assignment(html: &str, variable: &str) -> Option<String> {
    let pattern = format!(r#"{}\s*=\s*\"([^\"]*)\""#, regex::escape(variable));
    Regex::new(&pattern)
        .ok()?
        .captures(html)
        .and_then(|cap| cap.get(1).map(|m| decode_html(m.as_str())))
}

pub(super) fn vtinfo_dedup_key(loc: &RawStoreLocation) -> String {
    format!(
        "{}|{}|{}|{}",
        loc.name.to_lowercase(),
        loc.address_line1.as_deref().unwrap_or("").to_lowercase(),
        loc.city.as_deref().unwrap_or("").to_lowercase(),
        loc.state.as_deref().unwrap_or("").to_lowercase()
    )
}

fn strip_html(value: &str) -> String {
    let tag_re = Regex::new(r"<[^>]+>").expect("valid regex");
    decode_html(tag_re.replace_all(value, "").trim())
}

fn decode_html(value: &str) -> String {
    value
        .replace("\\/", "/")
        .replace("\\\"", "\"")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("\\u0026", "&")
        .trim()
        .to_string()
}
