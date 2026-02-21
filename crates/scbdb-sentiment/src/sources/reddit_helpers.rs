//! Reddit signal processing helpers: query building, brand matching, and post conversion.

use std::collections::HashSet;

use super::reddit::Post;
use crate::types::SentimentSignal;

pub(super) fn build_query_variants(brand_slug: &str, brand_name: &str) -> Vec<String> {
    let slug_query = brand_slug.replace('-', " ");
    let mut variants = vec![
        format!("\"{brand_name}\""),
        brand_name.to_string(),
        slug_query,
        format!("\"{brand_name}\" (thc OR cbd OR hemp OR cannabis)"),
        format!("\"{brand_name}\" (drink OR beverage OR seltzer)"),
    ];

    // Collapse duplicates while preserving order.
    let mut seen = HashSet::new();
    variants.retain(|q| seen.insert(q.to_lowercase()));
    variants
}

pub(super) fn build_brand_terms(brand_slug: &str, brand_name: &str) -> Vec<String> {
    let mut terms = vec![
        normalize_text_for_match(brand_name),
        normalize_text_for_match(&brand_name.replace('\'', "")),
        normalize_text_for_match(&brand_slug.replace('-', " ")),
    ];
    terms.retain(|t| !t.is_empty());
    terms.sort();
    terms.dedup();
    terms
}

pub(super) fn mentions_brand(text: &str, brand_terms: &[String]) -> bool {
    let normalized = normalize_text_for_match(text);
    let padded = format!(" {normalized} ");
    let compact = normalized.replace(' ', "");
    brand_terms.iter().any(|term| {
        if term.len() < 3 {
            return false;
        }
        let needle = format!(" {term} ");
        if padded.contains(&needle) {
            return true;
        }
        let compact_term = term.replace(' ', "");
        compact_term.len() >= 6 && compact.contains(&compact_term)
    })
}

fn normalize_text_for_match(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn to_signal(post: &Post, brand_slug: &str, kind: &str) -> Option<SentimentSignal> {
    let permalink = post.data.permalink.as_ref()?;
    let url = format!("https://reddit.com{permalink}");

    let text = if kind == "comment" {
        post.data
            .body
            .as_deref()
            .map(str::trim)
            .filter(|body| !body.is_empty() && *body != "[deleted]" && *body != "[removed]")
            .map(|body| body.chars().take(420).collect::<String>())?
    } else {
        let title = post
            .data
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())?;

        let text = match post.data.selftext.as_deref() {
            Some(body) if !body.is_empty() && body != "[deleted]" && body != "[removed]" => {
                let snippet: String = body.chars().take(280).collect();
                format!("{title} {snippet}")
            }
            _ => title.to_string(),
        };

        text
    };

    Some(SentimentSignal {
        text,
        url,
        source: if kind == "comment" {
            "reddit_comment".to_string()
        } else {
            "reddit_post".to_string()
        },
        brand_slug: brand_slug.to_string(),
        score: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_variants_include_brand_and_slug_forms() {
        let variants = build_query_variants("uncle-arnies", "Uncle Arnie's");
        assert!(variants.iter().any(|q| q.contains("\"Uncle Arnie's\"")));
        assert!(variants.iter().any(|q| q.contains("uncle arnies")));
        assert!(
            variants.iter().any(|q| q.contains("thc OR cbd")),
            "expected keyword-expanded query variant"
        );
    }

    #[test]
    fn mention_filter_matches_whole_phrase_and_rejects_near_miss() {
        let terms = vec!["uncle arnies".to_string()];
        assert!(mentions_brand("I love Uncle Arnie's iced tea", &terms));
        assert!(!mentions_brand("I love my uncle's iced tea", &terms));
    }
}
