//! Shopify cursor-based pagination via the `Link` response header.
//!
//! Shopify uses the cursor-based pagination pattern where the `Link` header
//! in each response carries URLs for adjacent pages. The cursor is encoded
//! as a `page_info` query parameter in the URL.
//!
//! ## Header format
//!
//! Single next link:
//! ```text
//! <https://shop.com/products.json?limit=250&page_info=CURSOR>; rel="next"
//! ```
//!
//! Combined previous and next:
//! ```text
//! <https://shop.com/products.json?limit=250&page_info=PREV>; rel="previous",
//! <https://shop.com/products.json?limit=250&page_info=NEXT>; rel="next"
//! ```

/// Parses a Shopify `Link` header value and extracts the `page_info` cursor
/// for the next page.
///
/// Returns `None` if:
/// - `link_header` is `None` (no header was present),
/// - there is no `rel="next"` segment (last page reached),
/// - the URL in the next segment has no `page_info` query parameter.
#[must_use]
pub fn extract_next_cursor(link_header: Option<&str>) -> Option<String> {
    let header = link_header?;

    // Split on "," to separate individual link directives.
    // Each segment looks like: `<URL>; rel="next"` (possibly with leading whitespace).
    for segment in header.split(',') {
        let segment = segment.trim();

        // Only process the "next" relation.
        if !segment.contains(r#"rel="next""#) {
            continue;
        }

        // Extract the URL from between the angle brackets.
        let url = extract_angle_bracket_url(segment)?;

        // Pull `page_info` from the query string.
        return extract_query_param(url, "page_info");
    }

    None
}

/// Extracts the URL between `<` and `>` in a link directive segment.
fn extract_angle_bracket_url(segment: &str) -> Option<&str> {
    let start = segment.find('<')? + 1;
    let end = segment.find('>')?;
    if start >= end {
        return None;
    }
    Some(&segment[start..end])
}

/// Extracts the value of a named query parameter from a URL string.
///
/// Does not decode percent-encoded characters — Shopify cursors are
/// base64url-encoded and contain no characters that require decoding.
fn extract_query_param(url: &str, param: &str) -> Option<String> {
    let query_start = url.find('?')? + 1;
    let query = &url[query_start..];

    let needle = format!("{param}=");
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix(needle.as_str()) {
            // Trim any fragment anchor that might trail the value.
            let value = value.split('#').next().unwrap_or(value);
            if !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_when_header_is_none() {
        assert!(extract_next_cursor(None).is_none());
    }

    #[test]
    fn returns_none_when_header_is_empty() {
        assert!(extract_next_cursor(Some("")).is_none());
    }

    #[test]
    fn extracts_cursor_from_single_next_link() {
        let header = r#"<https://drinkcann.com/products.json?limit=250&page_info=eyJsYXN0X2lkIjo2fQ>; rel="next""#;
        let cursor = extract_next_cursor(Some(header));
        assert_eq!(cursor.as_deref(), Some("eyJsYXN0X2lkIjo2fQ"));
    }

    #[test]
    fn extracts_cursor_from_combined_prev_next_link() {
        let header = concat!(
            r#"<https://drinkcann.com/products.json?limit=250&page_info=PREV_CURSOR>; rel="previous", "#,
            r#"<https://drinkcann.com/products.json?limit=250&page_info=NEXT_CURSOR>; rel="next""#
        );
        let cursor = extract_next_cursor(Some(header));
        assert_eq!(cursor.as_deref(), Some("NEXT_CURSOR"));
    }

    #[test]
    fn returns_none_when_only_previous_link_present() {
        let header = r#"<https://drinkcann.com/products.json?limit=250&page_info=PREV_CURSOR>; rel="previous""#;
        assert!(extract_next_cursor(Some(header)).is_none());
    }

    #[test]
    fn returns_none_when_no_page_info_in_next_url() {
        let header = r#"<https://drinkcann.com/products.json?limit=250>; rel="next""#;
        assert!(extract_next_cursor(Some(header)).is_none());
    }

    #[test]
    fn handles_extra_whitespace_between_segments() {
        // Some HTTP implementations add extra spaces after the comma.
        let header = concat!(
            r#"<https://example.com/products.json?limit=250&page_info=ABC>; rel="previous",   "#,
            r#"<https://example.com/products.json?limit=250&page_info=XYZ>; rel="next""#
        );
        let cursor = extract_next_cursor(Some(header));
        assert_eq!(cursor.as_deref(), Some("XYZ"));
    }

    #[test]
    fn extracts_cursor_when_page_info_is_not_the_first_query_param() {
        let header = r#"<https://drinkcann.com/products.json?limit=250&other=val&page_info=CURSOR123>; rel="next""#;
        let cursor = extract_next_cursor(Some(header));
        assert_eq!(cursor.as_deref(), Some("CURSOR123"));
    }

    // Internal helper tests
    #[test]
    fn extract_angle_bracket_url_happy_path() {
        let segment = r#"<https://example.com/foo?bar=baz>; rel="next""#;
        assert_eq!(
            extract_angle_bracket_url(segment),
            Some("https://example.com/foo?bar=baz")
        );
    }

    #[test]
    fn extract_angle_bracket_url_no_brackets_returns_none() {
        assert!(extract_angle_bracket_url("no brackets here").is_none());
    }

    #[test]
    fn extract_query_param_first_param() {
        assert_eq!(
            extract_query_param("https://x.com/p.json?page_info=ABC&limit=250", "page_info"),
            Some("ABC".to_owned())
        );
    }

    #[test]
    fn extract_query_param_second_param() {
        assert_eq!(
            extract_query_param("https://x.com/p.json?limit=250&page_info=XYZ", "page_info"),
            Some("XYZ".to_owned())
        );
    }

    #[test]
    fn extract_query_param_missing_returns_none() {
        assert!(extract_query_param("https://x.com/p.json?limit=250", "page_info").is_none());
    }
}
