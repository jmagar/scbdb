//! Brand-owned newsroom / press crawl source.

use std::collections::HashSet;

use regex::Regex;

use crate::types::SentimentSignal;

const INDEX_PATHS: [&str; 10] = [
    "/news",
    "/press",
    "/press-room",
    "/pressroom",
    "/blog",
    "/journal",
    "/media",
    "/company/news",
    "/blogs/news",
    "/blogs/journal",
];

const MAX_ARTICLES: usize = 10;

/// Crawl likely newsroom paths for a brand and return article-derived signals.
///
/// Returns empty when no usable base URL is available or no pages are found.
pub(crate) async fn fetch_brand_newsroom_signals(
    brand_slug: &str,
    brand_name: &str,
    brand_base_url: Option<&str>,
) -> Vec<SentimentSignal> {
    let Some(base) = normalize_base_url(brand_base_url) else {
        return Vec::new();
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(brand = brand_slug, error = %e, "failed to build newsroom client");
            return Vec::new();
        }
    };

    let mut candidate_article_urls: HashSet<String> = HashSet::new();

    for path in INDEX_PATHS {
        let index_url = format!("{base}{path}");
        let Ok(resp) = client.get(&index_url).send().await else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(body) = resp.text().await else {
            continue;
        };

        let links = extract_links(&body, &base);
        for url in links {
            if looks_like_article_url(&url, brand_name) {
                candidate_article_urls.insert(url);
                if candidate_article_urls.len() >= MAX_ARTICLES * 2 {
                    break;
                }
            }
        }
    }

    let mut urls: Vec<String> = candidate_article_urls.into_iter().collect();
    urls.sort();
    urls.truncate(MAX_ARTICLES);

    let mut signals = Vec::new();
    for article_url in urls {
        let Ok(resp) = client.get(&article_url).send().await else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(body) = resp.text().await else {
            continue;
        };

        let title = extract_title(&body);
        let description = extract_meta_description(&body);

        let text = if !title.is_empty() && !description.is_empty() {
            format!("{title} {description}")
        } else if !title.is_empty() {
            title
        } else if !description.is_empty() {
            description
        } else {
            continue;
        };

        signals.push(SentimentSignal {
            text,
            url: article_url,
            source: "brand_newsroom".to_string(),
            brand_slug: brand_slug.to_string(),
            score: 0.0,
        });
    }

    signals
}

fn normalize_base_url(base: Option<&str>) -> Option<String> {
    let raw = base?.trim();
    if raw.is_empty() {
        return None;
    }

    if raw.starts_with("http://") || raw.starts_with("https://") {
        Some(raw.trim_end_matches('/').to_string())
    } else {
        Some(format!("https://{}", raw.trim_end_matches('/')))
    }
}

fn extract_links(html: &str, base: &str) -> Vec<String> {
    let re = Regex::new(r#"(?i)href\s*=\s*[\"']([^\"'#?]+)[\"']"#).expect("valid href regex");
    let mut out = Vec::new();

    for cap in re.captures_iter(html) {
        let href = cap.get(1).map_or("", |m| m.as_str());
        if href.is_empty() {
            continue;
        }

        if href.starts_with("http://") || href.starts_with("https://") {
            out.push(href.to_string());
        } else if href.starts_with('/') {
            out.push(format!("{base}{href}"));
        }
    }

    out
}

fn looks_like_article_url(url: &str, brand_name: &str) -> bool {
    let lower = url.to_lowercase();
    let brand = brand_name.to_lowercase();

    if lower.contains("/product") || lower.contains("/shop") || lower.contains("/collections") {
        return false;
    }

    lower.contains("/news")
        || lower.contains("/press")
        || lower.contains("/blog")
        || lower.contains("/journal")
        || lower.contains(&brand.replace(' ', "-"))
}

fn extract_title(html: &str) -> String {
    let re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").expect("valid title regex");
    let Some(cap) = re.captures(html) else {
        return String::new();
    };
    clean_text(cap.get(1).map_or("", |m| m.as_str()))
}

fn extract_meta_description(html: &str) -> String {
    let re =
        Regex::new(r#"(?is)<meta\s+name=[\"']description[\"']\s+content=[\"'](.*?)[\"'][^>]*>"#)
            .expect("valid meta description regex");
    let Some(cap) = re.captures(html) else {
        return String::new();
    };
    clean_text(cap.get(1).map_or("", |m| m.as_str()))
}

fn clean_text(input: &str) -> String {
    let tags = Regex::new(r"(?is)<[^>]+>").expect("valid tags regex");
    let no_tags = tags.replace_all(input, " ");
    no_tags
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{clean_text, extract_links, looks_like_article_url};

    #[test]
    fn extract_links_expands_relative() {
        let html = r#"<a href='/news/a'>a</a><a href='https://x.com/news/b'>b</a>"#;
        let links = extract_links(html, "https://brand.com");
        assert!(links.iter().any(|x| x == "https://brand.com/news/a"));
        assert!(links.iter().any(|x| x == "https://x.com/news/b"));
    }

    #[test]
    fn article_heuristic_filters_shop_pages() {
        assert!(looks_like_article_url(
            "https://brand.com/news/new-launch",
            "Brand Name"
        ));
        assert!(!looks_like_article_url(
            "https://brand.com/collections/drinks",
            "Brand Name"
        ));
    }

    #[test]
    fn clean_text_strips_tags_and_normalizes_space() {
        assert_eq!(clean_text("<b>Hello</b>\n\nworld"), "Hello world");
    }
}
