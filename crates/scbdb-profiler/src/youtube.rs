//! `YouTube` Data API v3 client.
//!
//! Calls the `YouTube` search endpoint to list recent videos for a channel and
//! maps each result to a [`CollectedSignal`].

use crate::{error::ProfilerError, types::CollectedSignal};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

/// `YouTube` Data API v3 search endpoint.
const YOUTUBE_SEARCH_URL: &str = "https://www.googleapis.com/youtube/v3/search";

/// Maximum summary length stored per signal (characters).
const MAX_SUMMARY_LEN: usize = 2000;

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SearchResponse {
    items: Vec<SearchItem>,
    #[serde(rename = "nextPageToken")]
    #[allow(dead_code)] // reserved for future pagination
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchItem {
    id: VideoId,
    snippet: Snippet,
}

#[derive(Debug, Deserialize)]
struct VideoId {
    #[serde(rename = "videoId")]
    video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Snippet {
    #[serde(rename = "publishedAt")]
    published_at: String,
    title: String,
    description: String,
    thumbnails: Thumbnails,
}

#[derive(Debug, Deserialize)]
struct Thumbnails {
    high: Option<ThumbnailUrl>,
    medium: Option<ThumbnailUrl>,
    default: Option<ThumbnailUrl>,
}

#[derive(Debug, Deserialize)]
struct ThumbnailUrl {
    url: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Collect recent `YouTube` videos for a channel.
///
/// Returns one [`CollectedSignal`] per video. Items that lack a `videoId`
/// (e.g. playlist or channel results that slip through) are silently skipped.
///
/// # Errors
///
/// Returns [`ProfilerError::Http`] on network errors, HTTP-status errors,
/// or when the response body cannot be deserialized.
pub async fn collect_channel_signals(
    client: &Client,
    brand_id: i64,
    channel_id: &str,
    api_key: &str,
    max_results: u32,
) -> Result<Vec<CollectedSignal>, ProfilerError> {
    debug!(channel_id, brand_id, max_results, "fetching YouTube videos");

    let response = client
        .get(YOUTUBE_SEARCH_URL)
        .query(&[
            ("part", "snippet"),
            ("channelId", channel_id),
            ("type", "video"),
            ("order", "date"),
            ("maxResults", &max_results.to_string()),
            ("key", api_key),
        ])
        .send()
        .await?
        .error_for_status()?;

    let search: SearchResponse = response.json().await?;

    debug!(
        item_count = search.items.len(),
        channel_id, "parsed YouTube search results"
    );

    let signals = search
        .items
        .into_iter()
        .filter_map(|item| {
            let video_id = item.id.video_id?;

            Some(CollectedSignal {
                brand_id,
                signal_type: "youtube_video".to_string(),
                source_platform: Some("youtube".to_string()),
                source_url: Some(format!("https://www.youtube.com/watch?v={video_id}")),
                external_id: Some(video_id),
                title: Some(item.snippet.title),
                summary: Some(truncate(&item.snippet.description)),
                image_url: best_thumbnail(&item.snippet.thumbnails),
                view_count: None,
                like_count: None,
                comment_count: None,
                share_count: None,
                published_at: parse_youtube_datetime(&item.snippet.published_at),
            })
        })
        .collect();

    Ok(signals)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a `YouTube` ISO 8601 timestamp into `DateTime<Utc>`.
///
/// Returns `None` on invalid input rather than panicking.
fn parse_youtube_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Select the best available thumbnail, preferring high > medium > default.
fn best_thumbnail(thumbnails: &Thumbnails) -> Option<String> {
    thumbnails
        .high
        .as_ref()
        .or(thumbnails.medium.as_ref())
        .or(thumbnails.default.as_ref())
        .map(|t| t.url.clone())
}

/// Truncate a string to at most [`MAX_SUMMARY_LEN`] characters.
///
/// Operates on character boundaries so multi-byte content is never split
/// mid-codepoint.
fn truncate(s: &str) -> String {
    if s.chars().count() <= MAX_SUMMARY_LEN {
        s.to_string()
    } else {
        s.chars().take(MAX_SUMMARY_LEN).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    // -- parse_youtube_datetime --------------------------------------------

    #[test]
    fn parse_youtube_datetime_valid() {
        let result = parse_youtube_datetime("2024-01-15T10:00:00Z");
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn parse_youtube_datetime_with_offset() {
        let result = parse_youtube_datetime("2024-06-01T08:30:00+05:00");
        assert!(result.is_some());
        // Should be converted to UTC
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 3); // 08:30 +05:00 = 03:30 UTC
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn parse_youtube_datetime_invalid() {
        assert!(parse_youtube_datetime("not-a-date").is_none());
    }

    #[test]
    fn parse_youtube_datetime_empty() {
        assert!(parse_youtube_datetime("").is_none());
    }

    // -- best_thumbnail ----------------------------------------------------

    #[test]
    fn best_thumbnail_prefers_high() {
        let thumbnails = Thumbnails {
            high: Some(ThumbnailUrl {
                url: "high.jpg".to_string(),
            }),
            medium: Some(ThumbnailUrl {
                url: "medium.jpg".to_string(),
            }),
            default: Some(ThumbnailUrl {
                url: "default.jpg".to_string(),
            }),
        };
        assert_eq!(best_thumbnail(&thumbnails), Some("high.jpg".to_string()));
    }

    #[test]
    fn best_thumbnail_falls_back_to_medium() {
        let thumbnails = Thumbnails {
            high: None,
            medium: Some(ThumbnailUrl {
                url: "medium.jpg".to_string(),
            }),
            default: Some(ThumbnailUrl {
                url: "default.jpg".to_string(),
            }),
        };
        assert_eq!(best_thumbnail(&thumbnails), Some("medium.jpg".to_string()));
    }

    #[test]
    fn best_thumbnail_falls_back_to_default() {
        let thumbnails = Thumbnails {
            high: None,
            medium: None,
            default: Some(ThumbnailUrl {
                url: "default.jpg".to_string(),
            }),
        };
        assert_eq!(best_thumbnail(&thumbnails), Some("default.jpg".to_string()));
    }

    #[test]
    fn best_thumbnail_none_when_empty() {
        let thumbnails = Thumbnails {
            high: None,
            medium: None,
            default: None,
        };
        assert_eq!(best_thumbnail(&thumbnails), None);
    }

    // -- truncate ----------------------------------------------------------

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello"), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let long = "a".repeat(3000);
        let result = truncate(&long);
        assert_eq!(result.chars().count(), MAX_SUMMARY_LEN);
    }

    #[test]
    fn truncate_exact_boundary() {
        let exact = "b".repeat(MAX_SUMMARY_LEN);
        assert_eq!(truncate(&exact), exact);
    }

    // -- deserialization (offline) -----------------------------------------

    #[test]
    fn deserialize_search_response() {
        let json = r#"{
            "items": [
                {
                    "id": { "videoId": "abc123" },
                    "snippet": {
                        "publishedAt": "2024-01-15T10:00:00Z",
                        "title": "Test Video",
                        "description": "A test description.",
                        "thumbnails": {
                            "high": { "url": "https://img.youtube.com/vi/abc123/hqdefault.jpg" }
                        }
                    }
                }
            ],
            "nextPageToken": "CDIQAA"
        }"#;

        let resp: SearchResponse = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].id.video_id.as_deref(), Some("abc123"));
        assert_eq!(resp.items[0].snippet.title, "Test Video");
        assert_eq!(resp.next_page_token.as_deref(), Some("CDIQAA"));
    }

    #[test]
    fn deserialize_search_response_no_page_token() {
        let json = r#"{
            "items": []
        }"#;

        let resp: SearchResponse = serde_json::from_str(json).expect("should deserialize");
        assert!(resp.items.is_empty());
        assert!(resp.next_page_token.is_none());
    }

    #[test]
    fn deserialize_item_without_video_id() {
        // YouTube can return items where videoId is absent (e.g. playlist results)
        let json = r#"{
            "items": [
                {
                    "id": {},
                    "snippet": {
                        "publishedAt": "2024-01-15T10:00:00Z",
                        "title": "Playlist",
                        "description": "A playlist.",
                        "thumbnails": {}
                    }
                }
            ]
        }"#;

        let resp: SearchResponse = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(resp.items.len(), 1);
        assert!(resp.items[0].id.video_id.is_none());
    }
}
