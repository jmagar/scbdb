//! URL normalization, canonicalization, and sitemap utilities.

use regex::Regex;
use reqwest::Url;

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

pub(super) fn normalize_base_url(base: Option<&str>) -> Option<String> {
    let raw = base?.trim();
    if raw.is_empty() {
        return None;
    }

    if raw.starts_with("http://") || raw.starts_with("https://") {
        canonicalize_url(raw)
    } else {
        canonicalize_url(&format!("https://{raw}"))
    }
}

pub(super) fn newsroom_seed_urls(base: &str) -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(sitemap) = canonicalize_url(&format!("{base}/sitemap.xml")) {
        urls.push(sitemap);
    }
    urls.extend(newsroom_index_urls(base));
    urls
}

fn newsroom_index_urls(base: &str) -> Vec<String> {
    INDEX_PATHS
        .iter()
        .filter_map(|path| canonicalize_url(&format!("{base}{path}")))
        .collect()
}

pub(super) fn parse_robots_sitemaps(robots: &str, base: &str) -> Vec<String> {
    robots
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let (key, value) = trimmed.split_once(':')?;
            if !key.eq_ignore_ascii_case("sitemap") {
                return None;
            }
            resolve_and_canonicalize(value.trim(), base)
        })
        .collect()
}

pub(super) fn parse_sitemap_locs(xml: &str) -> Vec<String> {
    use super::html::clean_text;
    let re = Regex::new(r"(?is)<loc[^>]*>\s*(.*?)\s*</loc>").expect("valid loc regex");
    re.captures_iter(xml)
        .filter_map(|cap| cap.get(1).map(|m| clean_text(m.as_str())))
        .filter(|loc| !loc.is_empty())
        .collect()
}

pub(super) fn looks_like_sitemap_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    std::path::Path::new(&lower)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
        && lower.contains("sitemap")
}

pub(super) fn looks_like_article_url(url: &str, brand_name: &str) -> bool {
    let lower = url.to_lowercase();
    let brand = brand_name.to_lowercase().replace(' ', "-");

    let commerce_tokens = [
        "/product",
        "/products",
        "/shop",
        "/collections",
        "/cart",
        "/checkout",
        "/account",
        "/store",
    ];
    if commerce_tokens.iter().any(|token| lower.contains(token)) {
        return false;
    }

    lower.contains("/news")
        || lower.contains("/press")
        || lower.contains("/blog")
        || lower.contains("/journal")
        || lower.contains(&brand)
}

pub(super) fn resolve_and_canonicalize(raw: &str, base: &str) -> Option<String> {
    if raw.starts_with("http://") || raw.starts_with("https://") {
        canonicalize_url(raw)
    } else {
        let base_url = Url::parse(base).ok()?;
        let joined = base_url.join(raw).ok()?;
        canonicalize_url(joined.as_str())
    }
}

pub(super) fn canonicalize_url(raw: &str) -> Option<String> {
    let mut url = Url::parse(raw).ok()?;

    url.set_fragment(None);
    url.set_query(None);

    let path = url.path().to_string();
    if path.len() > 1 && path.ends_with('/') {
        url.set_path(path.trim_end_matches('/'));
    }

    Some(url.to_string())
}
