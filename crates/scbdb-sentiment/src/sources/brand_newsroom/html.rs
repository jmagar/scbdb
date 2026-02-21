//! HTML text extraction helpers for brand newsroom articles.

use regex::Regex;
use serde_json::json;
use serde_json::Value;

const MIN_TEXT_LEN: usize = 40;

pub(super) fn extract_links(html: &str, base: &str) -> Vec<String> {
    use super::urls::resolve_and_canonicalize;
    let re = Regex::new(r#"(?is)href\s*=\s*[\"']([^\"']+)[\"']"#).expect("valid href regex");
    re.captures_iter(html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .filter(|href| {
            !href.is_empty()
                && !href.starts_with('#')
                && !href.starts_with("mailto:")
                && !href.starts_with("javascript:")
        })
        .filter_map(|href| resolve_and_canonicalize(&href, base))
        .collect()
}

pub(super) fn extract_article_text(html: &str) -> Option<String> {
    if let Some(structured) = extract_json_ld_article_text(html) {
        return Some(structured);
    }

    let og_title = extract_og_title(html);
    let meta_description = extract_meta_description(html);
    let title = extract_title(html);
    let h1 = extract_h1(html);
    let first_paragraph = extract_first_paragraph(html);

    let selected = if !og_title.is_empty() && !meta_description.is_empty() {
        format!("{og_title} {meta_description}")
    } else if !title.is_empty() && !meta_description.is_empty() {
        format!("{title} {meta_description}")
    } else if !h1.is_empty() && !first_paragraph.is_empty() {
        format!("{h1} {first_paragraph}")
    } else {
        String::new()
    };

    let cleaned = clean_text(&selected);
    if cleaned.len() < MIN_TEXT_LEN {
        return None;
    }

    Some(cleaned)
}

pub(super) async fn extract_article_text_with_llm(
    client: &reqwest::Client,
    html: &str,
) -> Option<String> {
    if !llm_enabled() {
        return None;
    }

    let api_key = std::env::var("OPENAI_API_KEY").ok()?;
    let model =
        std::env::var("SENTIMENT_NEWSROOM_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let html_excerpt: String = html.chars().take(12_000).collect();
    let req_body = json!({
        "model": model,
        "response_format": { "type": "json_object" },
        "messages": [
            {
                "role": "system",
                "content": "Extract sentiment-relevant newsroom article content. Return JSON with keys: title, summary."
            },
            {
                "role": "user",
                "content": format!(
                    "Extract the main article title and a concise factual summary from this HTML. If no clear article is present, return empty strings.\n\nHTML:\n{}",
                    html_excerpt
                )
            }
        ],
        "temperature": 0.1
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&req_body)
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let body: Value = response.json().await.ok()?;
    let content = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(Value::as_str)?;

    parse_llm_json_response(content)
}

fn llm_enabled() -> bool {
    std::env::var("SENTIMENT_NEWSROOM_LLM_ENABLED")
        .ok()
        .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn parse_llm_json_response(content: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(content).ok()?;
    let title = parsed
        .get("title")
        .and_then(Value::as_str)
        .map(clean_text)
        .unwrap_or_default();
    let summary = parsed
        .get("summary")
        .and_then(Value::as_str)
        .map(clean_text)
        .unwrap_or_default();

    let combined = if !title.is_empty() && !summary.is_empty() {
        format!("{title} {summary}")
    } else if !title.is_empty() {
        title
    } else {
        summary
    };

    if combined.len() < MIN_TEXT_LEN {
        return None;
    }
    Some(combined)
}

fn extract_json_ld_article_text(html: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::parse_llm_json_response;

    #[test]
    fn parse_llm_json_response_combines_title_and_summary() {
        let raw = r#"{"title":"Cann expands distribution","summary":"The brand announced broader retail partnerships and seasonal launches this quarter."}"#;
        let text = parse_llm_json_response(raw).expect("expected parsed llm content");
        assert!(text.contains("Cann expands distribution"));
        assert!(text.contains("broader retail partnerships"));
    }

    #[test]
    fn parse_llm_json_response_rejects_short_payload() {
        let raw = r#"{"title":"Hi","summary":"Short"}"#;
        assert!(parse_llm_json_response(raw).is_none());
    }
}

fn extract_og_title(html: &str) -> String {
    let re = Regex::new(
        r#"(?is)<meta[^>]+property\s*=\s*[\"']og:title[\"'][^>]+content\s*=\s*[\"'](.*?)[\"'][^>]*>"#,
    )
    .expect("valid og title regex");

    if let Some(cap) = re.captures(html) {
        return clean_text(cap.get(1).map_or("", |m| m.as_str()));
    }

    let re_swapped = Regex::new(
        r#"(?is)<meta[^>]+content\s*=\s*[\"'](.*?)[\"'][^>]+property\s*=\s*[\"']og:title[\"'][^>]*>"#,
    )
    .expect("valid og title fallback regex");

    re_swapped
        .captures(html)
        .and_then(|cap| cap.get(1).map(|m| clean_text(m.as_str())))
        .unwrap_or_default()
}

fn extract_title(html: &str) -> String {
    let re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").expect("valid title regex");
    let Some(cap) = re.captures(html) else {
        return String::new();
    };
    clean_text(cap.get(1).map_or("", |m| m.as_str()))
}

fn extract_h1(html: &str) -> String {
    let re = Regex::new(r"(?is)<h1[^>]*>(.*?)</h1>").expect("valid h1 regex");
    let Some(cap) = re.captures(html) else {
        return String::new();
    };
    clean_text(cap.get(1).map_or("", |m| m.as_str()))
}

fn extract_first_paragraph(html: &str) -> String {
    let re = Regex::new(r"(?is)<p[^>]*>(.*?)</p>").expect("valid paragraph regex");
    for cap in re.captures_iter(html) {
        let candidate = clean_text(cap.get(1).map_or("", |m| m.as_str()));
        if candidate.len() >= 20 {
            return candidate;
        }
    }
    String::new()
}

fn extract_meta_description(html: &str) -> String {
    let re = Regex::new(
        r#"(?is)<meta[^>]+name\s*=\s*[\"']description[\"'][^>]+content\s*=\s*[\"'](.*?)[\"'][^>]*>"#,
    )
    .expect("valid meta description regex");

    if let Some(cap) = re.captures(html) {
        return clean_text(cap.get(1).map_or("", |m| m.as_str()));
    }

    let re_swapped = Regex::new(
        r#"(?is)<meta[^>]+content\s*=\s*[\"'](.*?)[\"'][^>]+name\s*=\s*[\"']description[\"'][^>]*>"#,
    )
    .expect("valid meta description fallback regex");

    re_swapped
        .captures(html)
        .and_then(|cap| cap.get(1).map(|m| clean_text(m.as_str())))
        .unwrap_or_default()
}

pub(super) fn clean_text(input: &str) -> String {
    let tags = Regex::new(r"(?is)<[^>]+>").expect("valid tags regex");
    let no_tags = tags.replace_all(input, " ");
    no_tags
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}
