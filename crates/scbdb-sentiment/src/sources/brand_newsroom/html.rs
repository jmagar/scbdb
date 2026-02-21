//! HTML text extraction helpers for brand newsroom articles.

use regex::Regex;
use serde_json::json;
use serde_json::Value;

use super::html_jsonld::extract_json_ld_article_text;

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

pub(super) async fn infer_newsroom_urls_with_llm(
    client: &reqwest::Client,
    base_url: &str,
    homepage_html: &str,
) -> Vec<String> {
    if !llm_enabled() {
        return Vec::new();
    }

    let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
        return Vec::new();
    };
    let model =
        std::env::var("SENTIMENT_NEWSROOM_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let html_excerpt: String = homepage_html.chars().take(12_000).collect();
    let href_candidates = extract_href_candidates(homepage_html);
    let href_block = href_candidates
        .iter()
        .take(200)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    let req_body = json!({
        "model": model,
        "response_format": { "type": "json_object" },
        "messages": [
            {
                "role": "system",
                "content": "Extract probable newsroom/press/blog URLs from brand homepage HTML. Return strict JSON with key: newsroom_urls (array of URL strings). Prefer same-domain links."
            },
            {
                "role": "user",
                "content": format!(
                    "Base URL: {base_url}\nReturn up to 8 best newsroom URLs.\nPrefer links containing: newsroom, press, media, investor/news, blog, journal, stories, announcements.\nIf none found, return an empty array.\n\nCandidate href values:\n{href_block}\n\nHTML excerpt:\n{html_excerpt}"
                )
            }
        ],
        "temperature": 0.0
    });

    let Ok(response) = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&req_body)
        .send()
        .await
    else {
        return Vec::new();
    };

    if !response.status().is_success() {
        return Vec::new();
    }

    let Ok(body): Result<Value, _> = response.json().await else {
        return Vec::new();
    };

    let Some(content) = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(Value::as_str)
    else {
        return Vec::new();
    };

    parse_llm_newsroom_urls_response(content)
}

fn extract_href_candidates(html: &str) -> Vec<String> {
    let re = Regex::new(r#"(?is)href\s*=\s*[\"']([^\"']+)[\"']"#).expect("valid href regex");
    re.captures_iter(html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .filter(|href| {
            !href.is_empty()
                && !href.starts_with('#')
                && !href.starts_with("mailto:")
                && !href.starts_with("javascript:")
        })
        .collect()
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

pub(super) fn parse_llm_newsroom_urls_response(content: &str) -> Vec<String> {
    let parsed: Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    parsed
        .get("newsroom_urls")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|u| !u.is_empty())
                .map(ToString::to_string)
                .take(8)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
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

    #[test]
    fn parse_llm_newsroom_urls_response_extracts_urls() {
        let raw =
            r#"{"newsroom_urls":["https://brand.com/news","/press","  https://brand.com/blog  "]}"#;
        let urls = super::parse_llm_newsroom_urls_response(raw);
        assert_eq!(urls.len(), 3);
        assert_eq!(urls[0], "https://brand.com/news");
        assert_eq!(urls[1], "/press");
        assert_eq!(urls[2], "https://brand.com/blog");
    }

    #[test]
    fn extract_href_candidates_filters_non_links() {
        let html = r##"
            <a href="/news">News</a>
            <a href="mailto:hello@brand.com">Mail</a>
            <a href="#top">Top</a>
            <a href="https://brand.com/press">Press</a>
        "##;
        let links = super::extract_href_candidates(html);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0], "/news");
        assert_eq!(links[1], "https://brand.com/press");
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
