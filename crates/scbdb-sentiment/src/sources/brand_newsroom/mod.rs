//! Brand-owned newsroom / press crawl source.

mod html;
mod urls;

use crate::types::SentimentSignal;

use html::{extract_article_text, extract_article_text_with_llm, extract_links};
use urls::{
    canonicalize_url, looks_like_article_url, looks_like_sitemap_url, newsroom_seed_urls,
    parse_robots_sitemaps, parse_sitemap_locs, resolve_and_canonicalize,
};

/// Upper bound for sitemap documents fetched per brand.
const MAX_SITEMAPS_PER_BRAND: usize = 12;
/// Upper bound for index pages fetched per brand.
const MAX_INDEX_PAGES_PER_BRAND: usize = 10;
/// Upper bound for article pages fetched per brand.
const MAX_ARTICLES_PER_BRAND: usize = 10;
/// Upper bound for LLM enrich calls per brand.
const MAX_LLM_ENRICH_CALLS_PER_BRAND: usize = 4;

/// Crawl likely newsroom paths for a brand and return article-derived signals.
///
/// Returns empty when no usable base URL is available or no pages are found.
#[allow(clippy::too_many_lines)]
pub(crate) async fn fetch_brand_newsroom_signals(
    brand_slug: &str,
    brand_name: &str,
    brand_base_url: Option<&str>,
) -> Vec<SentimentSignal> {
    let Some(base) = urls::normalize_base_url(brand_base_url) else {
        return Vec::new();
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                brand = brand_slug,
                source = "brand_newsroom",
                error = %e,
                "failed to build newsroom client"
            );
            return Vec::new();
        }
    };

    let mut candidate_article_urls: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    // 1) robots.txt sitemap refs
    let robots_url = format!("{base}/robots.txt");
    let mut sitemap_queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    let mut seen_sitemaps: std::collections::HashSet<String> = std::collections::HashSet::new();

    match client.get(&robots_url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(body) => {
                for sitemap in parse_robots_sitemaps(&body, &base) {
                    if seen_sitemaps.insert(sitemap.clone()) {
                        sitemap_queue.push_back(sitemap);
                    }
                }
            }
            Err(e) => {
                tracing::debug!(
                    brand = brand_slug,
                    source = "brand_newsroom",
                    url = %robots_url,
                    error = %e,
                    "failed reading robots.txt"
                );
            }
        },
        Ok(_) => {}
        Err(e) => {
            tracing::debug!(
                brand = brand_slug,
                source = "brand_newsroom",
                url = %robots_url,
                error = %e,
                "failed fetching robots.txt"
            );
        }
    }

    // 2) /sitemap.xml
    if let Some(default_sitemap) = canonicalize_url(&format!("{base}/sitemap.xml")) {
        if seen_sitemaps.insert(default_sitemap.clone()) {
            sitemap_queue.push_back(default_sitemap);
        }
    }

    let mut fetched_sitemaps = 0;
    while fetched_sitemaps < MAX_SITEMAPS_PER_BRAND {
        let Some(sitemap_url) = sitemap_queue.pop_front() else {
            break;
        };
        fetched_sitemaps += 1;

        let body = match client.get(&sitemap_url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(body) => body,
                Err(e) => {
                    tracing::debug!(
                        brand = brand_slug,
                        source = "brand_newsroom",
                        url = %sitemap_url,
                        error = %e,
                        "failed reading sitemap"
                    );
                    continue;
                }
            },
            Ok(_) => continue,
            Err(e) => {
                tracing::debug!(
                    brand = brand_slug,
                    source = "brand_newsroom",
                    url = %sitemap_url,
                    error = %e,
                    "failed fetching sitemap"
                );
                continue;
            }
        };

        for loc in parse_sitemap_locs(&body) {
            let Some(canonical_loc) = resolve_and_canonicalize(&loc, &base) else {
                continue;
            };

            if looks_like_sitemap_url(&canonical_loc) {
                if seen_sitemaps.len() < MAX_SITEMAPS_PER_BRAND
                    && seen_sitemaps.insert(canonical_loc.clone())
                {
                    sitemap_queue.push_back(canonical_loc);
                }
                continue;
            }

            if looks_like_article_url(&canonical_loc, brand_name) {
                candidate_article_urls.insert(canonical_loc);
            }
        }
    }

    // 3) newsroom path seeds
    for index_url in newsroom_seed_urls(&base)
        .into_iter()
        .filter(|url| !looks_like_sitemap_url(url))
        .take(MAX_INDEX_PAGES_PER_BRAND)
    {
        let body = match client.get(&index_url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(body) => body,
                Err(e) => {
                    tracing::debug!(
                        brand = brand_slug,
                        source = "brand_newsroom",
                        url = %index_url,
                        error = %e,
                        "failed reading index page"
                    );
                    continue;
                }
            },
            Ok(_) => continue,
            Err(e) => {
                tracing::debug!(
                    brand = brand_slug,
                    source = "brand_newsroom",
                    url = %index_url,
                    error = %e,
                    "failed fetching index page"
                );
                continue;
            }
        };

        for url in extract_links(&body, &base) {
            if looks_like_article_url(&url, brand_name) {
                candidate_article_urls.insert(url);
            }
        }
    }

    let mut article_urls: Vec<String> = candidate_article_urls.into_iter().collect();
    article_urls.sort();
    article_urls.truncate(MAX_ARTICLES_PER_BRAND);

    let mut signals = Vec::new();
    let mut llm_enrich_calls = 0usize;
    for article_url in article_urls {
        let body = match client.get(&article_url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(body) => body,
                Err(_) => continue,
            },
            _ => continue,
        };

        let deterministic_text = extract_article_text(&body);
        let llm_text = if llm_enrich_calls < MAX_LLM_ENRICH_CALLS_PER_BRAND {
            llm_enrich_calls += 1;
            extract_article_text_with_llm(&client, &body).await
        } else {
            None
        };

        let Some(text) = merge_extracted_text(deterministic_text, llm_text) else {
            continue;
        };

        let url = canonicalize_url(&article_url).unwrap_or(article_url);
        signals.push(SentimentSignal {
            text,
            url,
            source: "brand_newsroom".to_string(),
            brand_slug: brand_slug.to_string(),
            score: 0.0,
        });
    }

    signals
}

fn merge_extracted_text(deterministic: Option<String>, llm: Option<String>) -> Option<String> {
    match (deterministic, llm) {
        (Some(a), Some(b)) => {
            if a == b || a.contains(&b) {
                Some(a)
            } else if b.contains(&a) {
                Some(b)
            } else {
                Some(format!("{a} {b}"))
            }
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{
        html::{clean_text, extract_article_text, extract_links},
        urls::{
            canonicalize_url, looks_like_article_url, newsroom_seed_urls, normalize_base_url,
            parse_sitemap_locs,
        },
    };

    #[test]
    fn normalize_base_url_handles_core_cases() {
        assert_eq!(
            normalize_base_url(Some("brand.com/")),
            Some("https://brand.com/".to_string())
        );
        assert_eq!(
            normalize_base_url(Some("https://brand.com/")),
            Some("https://brand.com/".to_string())
        );
        assert_eq!(
            normalize_base_url(Some("http://brand.com/path/")),
            Some("http://brand.com/path".to_string())
        );
    }

    #[test]
    fn newsroom_seed_urls_include_sitemap_and_standard_paths() {
        let seeds = newsroom_seed_urls("https://brand.com");
        assert!(seeds.iter().any(|u| u == "https://brand.com/sitemap.xml"));
        assert!(seeds.iter().any(|u| u == "https://brand.com/news"));
        assert!(seeds.iter().any(|u| u == "https://brand.com/press"));
        assert!(seeds.iter().any(|u| u == "https://brand.com/blog"));
    }

    #[test]
    fn parse_sitemap_locs_supports_urlset_and_sitemapindex() {
        let urlset = r#"
            <urlset>
                <url><loc>https://brand.com/news/a</loc></url>
                <url><loc>https://brand.com/news/b</loc></url>
            </urlset>
        "#;
        let sitemapindex = r#"
            <sitemapindex>
                <sitemap><loc>https://brand.com/sitemaps/news.xml</loc></sitemap>
                <sitemap><loc>https://brand.com/sitemaps/blog.xml</loc></sitemap>
            </sitemapindex>
        "#;

        let from_urlset = parse_sitemap_locs(urlset);
        let from_index = parse_sitemap_locs(sitemapindex);

        assert!(from_urlset.iter().any(|u| u == "https://brand.com/news/a"));
        assert!(from_urlset.iter().any(|u| u == "https://brand.com/news/b"));
        assert!(from_index
            .iter()
            .any(|u| u == "https://brand.com/sitemaps/news.xml"));
        assert!(from_index
            .iter()
            .any(|u| u == "https://brand.com/sitemaps/blog.xml"));
    }

    #[test]
    fn extract_links_resolves_relative_and_absolute() {
        let html = r#"
            <a href='/news/a'>a</a>
            <a href='https://brand.com/news/b?utm=1#top'>b</a>
        "#;

        let links = extract_links(html, "https://brand.com");
        assert!(links.iter().any(|x| x == "https://brand.com/news/a"));
        assert!(links.iter().any(|x| x == "https://brand.com/news/b"));
    }

    #[test]
    fn looks_like_article_url_rejects_commerce_paths() {
        assert!(looks_like_article_url(
            "https://brand.com/news/new-launch",
            "Brand Name"
        ));
        assert!(!looks_like_article_url(
            "https://brand.com/products/new-launch",
            "Brand Name"
        ));
        assert!(!looks_like_article_url(
            "https://brand.com/collections/drinks",
            "Brand Name"
        ));
        assert!(!looks_like_article_url(
            "https://brand.com/shop/sparkling-water",
            "Brand Name"
        ));
    }

    #[test]
    fn extraction_fallback_chain_prefers_og_then_title_then_h1() {
        let html_og = r#"
            <html>
                <head>
                    <meta property='og:title' content='OG Launch Story'>
                    <meta name='description' content='A long enough description for the newsroom signal extraction test.'>
                    <title>Document Title</title>
                </head>
                <body><h1>Fallback Heading</h1><p>Fallback paragraph content for extraction.</p></body>
            </html>
        "#;
        let extracted_og = extract_article_text(html_og).expect("expected og extraction");
        assert!(extracted_og.starts_with("OG Launch Story"));

        let html_title = r#"
            <html>
                <head>
                    <title>Title Launch Story</title>
                    <meta name='description' content='A long enough description for title fallback extraction coverage.'>
                </head>
            </html>
        "#;
        let extracted_title = extract_article_text(html_title).expect("expected title extraction");
        assert!(extracted_title.starts_with("Title Launch Story"));

        let html_h1 = r#"
            <html>
                <body>
                    <h1>Heading Launch Story</h1>
                    <p>Paragraph content that is long enough to pass minimum text checks.</p>
                </body>
            </html>
        "#;
        let extracted_h1 = extract_article_text(html_h1).expect("expected h1 extraction");
        assert!(extracted_h1.starts_with("Heading Launch Story"));
    }

    #[test]
    fn extraction_rejects_short_text_payloads() {
        let html = r#"<html><head><title>Hi</title><meta name='description' content='Too short'></head></html>"#;
        assert!(extract_article_text(html).is_none());
    }

    #[test]
    fn extraction_uses_json_ld_article_when_present() {
        let html = r#"
            <html>
                <head>
                    <script type="application/ld+json">
                    {
                      "@context":"https://schema.org",
                      "@type":"NewsArticle",
                      "headline":"Cann opens new production line",
                      "description":"The company announced expanded capacity and multi-state distribution this quarter."
                    }
                    </script>
                </head>
                <body><h1>Fallback heading</h1><p>fallback text</p></body>
            </html>
        "#;

        let extracted = extract_article_text(html).expect("expected json-ld extraction");
        assert!(extracted.starts_with("Cann opens new production line"));
    }

    #[test]
    fn extraction_uses_json_ld_graph_article_nodes() {
        let html = r#"
            <html>
                <head>
                    <script type="application/ld+json">
                    {
                      "@context":"https://schema.org",
                      "@graph":[
                        { "@type":"Organization", "name":"Cann" },
                        {
                          "@type":"PressRelease",
                          "headline":"Cann announces seasonal release",
                          "description":"Seasonal SKU expansion with new retail partnerships and broader distribution footprint."
                        }
                      ]
                    }
                    </script>
                </head>
            </html>
        "#;

        let extracted = extract_article_text(html).expect("expected graph json-ld extraction");
        assert!(extracted.contains("Cann announces seasonal release"));
    }

    #[test]
    fn canonicalization_drops_query_fragment_and_trailing_slash() {
        assert_eq!(
            canonicalize_url("https://brand.com/news/launch/?utm=campaign#section"),
            Some("https://brand.com/news/launch".to_string())
        );
    }

    #[test]
    fn clean_text_strips_tags_and_normalizes_space() {
        assert_eq!(clean_text("<b>Hello</b>\n\nworld"), "Hello world");
    }
}
