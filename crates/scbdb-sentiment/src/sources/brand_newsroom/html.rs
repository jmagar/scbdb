//! HTML text extraction helpers for brand newsroom articles.

use regex::Regex;

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
