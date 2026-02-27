use std::sync::LazyLock;

use regex::Regex;

use crate::client::extract_store_origin;
use crate::ScraperError;

static IMG_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<img\b[^>]*>").expect("valid regex"));
static LINK_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<link\b[^>]*>").expect("valid regex"));
static META_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<meta\b[^>]*>").expect("valid regex"));
static SIZES_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\d+)\s*x\s*(\d+)").expect("valid sizes regex"));

const BROWSER_FALLBACK_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Copy)]
enum CandidateSource {
    OgLogo,
    ImgLogo,
    LinkIcon,
    OgImage,
}

#[derive(Debug, Clone)]
struct LogoCandidate {
    url: String,
    source: CandidateSource,
    width: Option<i32>,
    height: Option<i32>,
}

/// Best-effort logo extraction from a Shopify storefront homepage.
///
/// Candidate ranking strongly prefers logo-like assets and de-prioritizes
/// favicon-sized icons (`.ico`, 32x32, `apple-touch-icon`, `favicon`).
///
/// # Errors
///
/// Returns [`ScraperError`] if the HTTP client cannot be built or the request fails.
pub async fn fetch_brand_logo_url(
    shop_url: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<Option<String>, ScraperError> {
    let origin = extract_store_origin(shop_url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let mut user_agents = vec![user_agent.to_string()];
    if !user_agents.iter().any(|ua| ua == BROWSER_FALLBACK_UA) {
        user_agents.push(BROWSER_FALLBACK_UA.to_string());
    }

    for ua in user_agents {
        let Ok(response) = client
            .get(&origin)
            .header(reqwest::header::USER_AGENT, ua)
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
            .send()
            .await
        else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        let Ok(body) = response.text().await else {
            continue;
        };
        if let Some(url) = extract_logo_candidate(&origin, &body) {
            return Ok(Some(url));
        }
    }

    Ok(None)
}

fn extract_logo_candidate(base_url: &str, html: &str) -> Option<String> {
    let mut candidates = collect_candidates(base_url, html);
    candidates.sort_by_key(score_candidate);
    candidates.last().map(|c| c.url.clone())
}

fn collect_candidates(base_url: &str, html: &str) -> Vec<LogoCandidate> {
    let mut candidates: Vec<LogoCandidate> = Vec::new();

    if let Some(raw) = find_meta_content(html, "property", "og:logo")
        .and_then(|raw| absolutize_url(base_url, &raw))
    {
        candidates.push(LogoCandidate {
            url: raw,
            source: CandidateSource::OgLogo,
            width: None,
            height: None,
        });
    }

    for m in IMG_TAG_RE.find_iter(html) {
        let tag = m.as_str();
        let marker = [
            extract_attr(tag, "class"),
            extract_attr(tag, "id"),
            extract_attr(tag, "alt"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

        if !marker.contains("logo") {
            continue;
        }

        let Some(src) = extract_attr(tag, "src").and_then(|raw| absolutize_url(base_url, &raw))
        else {
            continue;
        };
        let width = extract_attr(tag, "width").and_then(|v| v.parse::<i32>().ok());
        let height = extract_attr(tag, "height").and_then(|v| v.parse::<i32>().ok());
        candidates.push(LogoCandidate {
            url: src,
            source: CandidateSource::ImgLogo,
            width,
            height,
        });
    }

    for m in LINK_TAG_RE.find_iter(html) {
        let tag = m.as_str();
        let Some(rel_raw) = extract_attr(tag, "rel") else {
            continue;
        };
        let rel = rel_raw.to_ascii_lowercase();
        if !rel.contains("icon") {
            continue;
        }
        let Some(href) = extract_attr(tag, "href").and_then(|raw| absolutize_url(base_url, &raw))
        else {
            continue;
        };
        let (width, height) = extract_attr(tag, "sizes")
            .as_deref()
            .and_then(parse_sizes_attr)
            .unwrap_or((None, None));
        candidates.push(LogoCandidate {
            url: href,
            source: CandidateSource::LinkIcon,
            width,
            height,
        });
    }

    if let Some(raw) = find_meta_content(html, "property", "og:image")
        .and_then(|raw| absolutize_url(base_url, &raw))
    {
        candidates.push(LogoCandidate {
            url: raw,
            source: CandidateSource::OgImage,
            width: None,
            height: None,
        });
    }

    candidates
}

#[allow(clippy::case_sensitive_file_extension_comparisons)] // url_lower is already lowercased
fn score_candidate(candidate: &LogoCandidate) -> i32 {
    let mut score = match candidate.source {
        CandidateSource::OgLogo => 600,
        CandidateSource::ImgLogo => 500,
        CandidateSource::OgImage => 340,
        CandidateSource::LinkIcon => 80,
    };

    let url_lower = candidate.url.to_ascii_lowercase();

    score += if url_lower.ends_with(".svg")
        || url_lower.contains(".svg?")
        || url_lower.contains("image/svg+xml")
    {
        120
    } else if url_lower.ends_with(".png") || url_lower.contains(".png?") {
        100
    } else if url_lower.ends_with(".webp") || url_lower.contains(".webp?") {
        70
    } else if url_lower.ends_with(".jpg")
        || url_lower.contains(".jpg?")
        || url_lower.ends_with(".jpeg")
        || url_lower.contains(".jpeg?")
    {
        50
    } else if url_lower.ends_with(".ico") || url_lower.contains(".ico?") {
        -260
    } else {
        0
    };

    if url_lower.contains("favicon") {
        score -= 220;
    }
    if url_lower.contains("apple-touch-icon") {
        score -= 130;
    }
    if url_lower.contains("logo") {
        score += 80;
    }
    if matches!(candidate.source, CandidateSource::LinkIcon) {
        score -= 110;
    }

    if let (Some(w), Some(h)) = (candidate.width, candidate.height) {
        let min_dim = w.min(h);
        if min_dim <= 32 {
            score -= 260;
        } else if min_dim <= 64 {
            score -= 160;
        } else if min_dim <= 96 {
            score -= 70;
        } else if min_dim >= 220 {
            score += 90;
        } else if min_dim >= 120 {
            score += 45;
        }
    }

    score
}

fn parse_sizes_attr(value: &str) -> Option<(Option<i32>, Option<i32>)> {
    let caps = SIZES_RE.captures(value)?;
    let width = caps.get(1).and_then(|m| m.as_str().parse::<i32>().ok());
    let height = caps.get(2).and_then(|m| m.as_str().parse::<i32>().ok());
    Some((width, height))
}

fn find_meta_content(html: &str, key_attr: &str, key_value: &str) -> Option<String> {
    let result = META_TAG_RE.find_iter(html).find_map(|m| {
        let tag = m.as_str();
        let key = extract_attr(tag, key_attr)?;
        if key.eq_ignore_ascii_case(key_value) {
            extract_attr(tag, "content")
        } else {
            None
        }
    });
    result
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"(?is)\b{}\s*=\s*["']([^"']+)["']"#, regex::escape(attr));
    let re = Regex::new(&pattern).expect("valid attr regex");
    re.captures(tag)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
}

fn absolutize_url(base_url: &str, candidate: &str) -> Option<String> {
    let candidate = candidate.replace("&amp;", "&");
    let base = reqwest::Url::parse(base_url).ok()?;
    base.join(&candidate).ok().map(|u| u.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_logo_from_img_tag() {
        let html = r#"<html><body><img class="site-logo" src="/assets/logo.png" width="240" height="80"></body></html>"#;
        let got = extract_logo_candidate("https://example.com", html);
        assert_eq!(got.as_deref(), Some("https://example.com/assets/logo.png"));
    }

    #[test]
    fn prefers_logo_over_favicon_icon() {
        let html = r#"
            <html>
              <head>
                <link rel="icon" href="/favicon.ico" sizes="32x32">
              </head>
              <body>
                <img id="main-logo" src="/assets/brand-logo.svg" width="320" height="80">
              </body>
            </html>
        "#;
        let got = extract_logo_candidate("https://example.com", html);
        assert_eq!(
            got.as_deref(),
            Some("https://example.com/assets/brand-logo.svg")
        );
    }

    #[test]
    fn prefers_og_logo_over_og_image() {
        let html = r#"
            <meta property="og:logo" content="https://cdn.example.com/logo.svg">
            <meta property="og:image" content="https://cdn.example.com/hero.jpg">
        "#;
        let got = extract_logo_candidate("https://example.com", html);
        assert_eq!(got.as_deref(), Some("https://cdn.example.com/logo.svg"));
    }

    #[test]
    fn falls_back_to_icon_link() {
        let html = r#"<html><head><link rel="icon" href="https://cdn.example.com/favicon.png"></head></html>"#;
        let got = extract_logo_candidate("https://example.com", html);
        assert_eq!(got.as_deref(), Some("https://cdn.example.com/favicon.png"));
    }
}
