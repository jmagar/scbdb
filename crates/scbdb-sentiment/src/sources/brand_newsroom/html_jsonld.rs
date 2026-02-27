//! JSON-LD structured data extraction for newsroom article text.

use regex::Regex;
use serde_json::Value;

const MIN_TEXT_LEN: usize = 40;

pub(super) fn extract_json_ld_article_text(html: &str) -> Option<String> {
    let script_re = Regex::new(
        r#"(?is)<script[^>]*type\s*=\s*["']application/ld\+json["'][^>]*>(.*?)</script>"#,
    )
    .expect("valid json-ld script regex");

    let mut best = String::new();

    for cap in script_re.captures_iter(html) {
        let raw = cap.get(1).map_or("", |m| m.as_str()).trim();
        if raw.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(raw) else {
            continue;
        };

        if let Some(candidate) = extract_from_json_ld_value(&value) {
            if candidate.len() > best.len() {
                best = candidate;
            }
        }
    }

    if best.len() < MIN_TEXT_LEN {
        return None;
    }
    Some(best)
}

fn extract_from_json_ld_value(value: &Value) -> Option<String> {
    let mut candidates = Vec::new();
    collect_json_ld_candidates(value, &mut candidates);
    candidates.into_iter().max_by_key(String::len)
}

fn collect_json_ld_candidates(value: &Value, out: &mut Vec<String>) {
    use super::html::clean_text;
    match value {
        Value::Object(map) => {
            if looks_like_article_node(map.get("@type")) {
                let title = map
                    .get("headline")
                    .or_else(|| map.get("name"))
                    .and_then(Value::as_str)
                    .map(clean_text)
                    .unwrap_or_default();

                let detail = map
                    .get("description")
                    .or_else(|| map.get("articleBody"))
                    .and_then(Value::as_str)
                    .map(clean_text)
                    .unwrap_or_default();

                let combined = if !title.is_empty() && !detail.is_empty() {
                    format!("{title} {detail}")
                } else if !title.is_empty() {
                    title
                } else {
                    detail
                };

                if !combined.is_empty() {
                    out.push(combined);
                }
            }

            for child in map.values() {
                collect_json_ld_candidates(child, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_json_ld_candidates(child, out);
            }
        }
        _ => {}
    }
}

fn contains_article_token(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("article")
        || lower.contains("newsarticle")
        || lower.contains("blogposting")
        || lower.contains("pressrelease")
}

fn looks_like_article_node(node_type: Option<&Value>) -> bool {
    let Some(node_type) = node_type else {
        return false;
    };

    match node_type {
        Value::String(s) => contains_article_token(s),
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .any(contains_article_token),
        _ => false,
    }
}
