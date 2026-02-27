//! X (Twitter) profile signal collector -- CLI wrapper.
//!
//! Wraps a local CLI tool (`t` ruby gem) to fetch recent tweets without
//! requiring X API keys. The synchronous [`std::process::Command`] call is
//! wrapped in [`tokio::task::spawn_blocking`] to avoid blocking the async
//! runtime.
//!
//! **Graceful degradation**: if the CLI tool is not installed or returns an
//! error, this module returns `Ok(vec![])` with a `tracing::warn` log rather
//! than propagating an error. Twitter data is best-effort.

use crate::{error::ProfilerError, types::CollectedSignal};
use chrono::{DateTime, Utc};
use std::process::Command;
use tracing::warn;

/// Maximum summary length stored per signal (characters).
const MAX_SUMMARY_LEN: usize = 2000;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Collect recent tweets from an X profile.
///
/// Runs the external CLI tool to fetch tweets -- wraps the synchronous
/// process call in `spawn_blocking` to avoid blocking the async runtime.
///
/// # Parameters
/// - `brand_id`: brand to associate with the collected signals
/// - `handle`: X/Twitter handle WITHOUT the `@` sign (e.g., `"drinkwild"`)
/// - `limit`: maximum number of tweets to collect (1--200)
///
/// # Errors
///
/// Returns [`ProfilerError::Other`] if `spawn_blocking` itself fails (join
/// error). CLI tool absence or failure is **not** an error -- those cases
/// return an empty `Vec` with a warning log.
pub async fn collect_profile_signals(
    brand_id: i64,
    handle: &str,
    limit: u32,
) -> Result<Vec<CollectedSignal>, ProfilerError> {
    let handle = handle.to_string();

    let tweets = tokio::task::spawn_blocking(move || run_twitter_cli(&handle, limit))
        .await
        .map_err(|e| ProfilerError::Other(format!("spawn_blocking join error: {e}")))?;

    Ok(tweets
        .into_iter()
        .map(|t| tweet_to_signal(brand_id, t))
        .collect())
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// A parsed tweet from the CLI output.
#[derive(Debug)]
struct Tweet {
    id: String,
    text: String,
    created_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// CLI execution
// ---------------------------------------------------------------------------

/// Run the `t` CLI tool and parse output into tweets.
///
/// The `t` ruby gem (<https://github.com/sferik/t>) outputs TSV-style lines:
///
/// ```text
/// ID\t@handle\t2024-01-15 10:00:00\tTweet text here
/// ```
///
/// If the tool is not installed or returns a non-zero exit code, we degrade
/// gracefully and return an empty list.
fn run_twitter_cli(handle: &str, limit: u32) -> Vec<Tweet> {
    let output = Command::new("t")
        .args(["timeline", "-n", &limit.to_string(), handle])
        .output();

    match output {
        Err(_) => {
            // CLI tool not available -- not an error for us.
            warn!(handle, "twitter CLI tool `t` not found, skipping");
            vec![]
        }
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                handle,
                stderr = %stderr,
                "twitter CLI returned non-zero exit code, skipping"
            );
            vec![]
        }
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_cli_output(&stdout)
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse TSV-style output from the `t` CLI into [`Tweet`] structs.
///
/// Expected format per line (tab-separated, 4 columns):
///
/// ```text
/// ID\t@handle\t2024-01-15 10:00:00\tTweet text here
/// ```
///
/// Lines with fewer than 4 tab-separated columns are silently skipped.
/// Lines with empty `id` or `text` are also skipped.
fn parse_cli_output(output: &str) -> Vec<Tweet> {
    let mut tweets = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(4, '\t').collect();
        if parts.len() < 4 {
            continue; // skip malformed lines
        }

        let id = parts[0].trim().to_string();
        let created_at =
            chrono::NaiveDateTime::parse_from_str(parts[2].trim(), "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.and_utc());
        let text = parts[3].trim().to_string();

        if !id.is_empty() && !text.is_empty() {
            tweets.push(Tweet {
                id,
                text,
                created_at,
            });
        }
    }

    tweets
}

// ---------------------------------------------------------------------------
// Signal mapping
// ---------------------------------------------------------------------------

/// Convert a parsed [`Tweet`] into a [`CollectedSignal`].
fn tweet_to_signal(brand_id: i64, tweet: Tweet) -> CollectedSignal {
    CollectedSignal {
        brand_id,
        signal_type: "tweet".to_string(),
        source_platform: Some("twitter".to_string()),
        source_url: None,
        external_id: Some(tweet.id),
        title: None,
        summary: Some(truncate(&tweet.text)),
        image_url: None,
        view_count: None,
        like_count: None,
        comment_count: None,
        share_count: None,
        published_at: tweet.created_at,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

    // -- parse_cli_output ---------------------------------------------------

    #[test]
    fn parse_cli_output_valid_line() {
        let output = "123456\t@drinkwild\t2024-01-15 10:00:00\tHello world tweet";
        let tweets = parse_cli_output(output);
        assert_eq!(tweets.len(), 1);
        assert_eq!(tweets[0].id, "123456");
        assert_eq!(tweets[0].text, "Hello world tweet");
        assert!(tweets[0].created_at.is_some());

        let dt = tweets[0].created_at.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn parse_cli_output_multiple_lines() {
        let output = "\
            111\t@brand\t2024-01-01 08:00:00\tFirst tweet\n\
            222\t@brand\t2024-01-02 09:00:00\tSecond tweet";
        let tweets = parse_cli_output(output);
        assert_eq!(tweets.len(), 2);
        assert_eq!(tweets[0].id, "111");
        assert_eq!(tweets[1].id, "222");
    }

    #[test]
    fn parse_cli_output_skips_malformed() {
        let output = "only-one-field";
        let tweets = parse_cli_output(output);
        assert!(tweets.is_empty());
    }

    #[test]
    fn parse_cli_output_skips_partial_tabs() {
        let output = "id\thandle\ttimestamp"; // only 3 columns
        let tweets = parse_cli_output(output);
        assert!(tweets.is_empty());
    }

    #[test]
    fn parse_cli_output_empty_input() {
        let tweets = parse_cli_output("");
        assert!(tweets.is_empty());
    }

    #[test]
    fn parse_cli_output_skips_empty_id() {
        let output = "\t@brand\t2024-01-01 08:00:00\tSome text";
        let tweets = parse_cli_output(output);
        assert!(tweets.is_empty());
    }

    #[test]
    fn parse_cli_output_skips_empty_text() {
        let output = "123\t@brand\t2024-01-01 08:00:00\t";
        let tweets = parse_cli_output(output);
        assert!(tweets.is_empty());
    }

    #[test]
    fn parse_cli_output_bad_datetime_still_collects() {
        let output = "999\t@brand\tnot-a-date\tTweet with bad date";
        let tweets = parse_cli_output(output);
        assert_eq!(tweets.len(), 1);
        assert_eq!(tweets[0].id, "999");
        assert!(tweets[0].created_at.is_none());
    }

    // -- tweet_to_signal ----------------------------------------------------

    #[test]
    fn tweet_to_signal_maps_correctly() {
        let tweet = Tweet {
            id: "99887766".to_string(),
            text: "Test tweet".to_string(),
            created_at: None,
        };
        let signal = tweet_to_signal(42, tweet);
        assert_eq!(signal.signal_type, "tweet");
        assert_eq!(signal.source_platform, Some("twitter".to_string()));
        assert_eq!(signal.external_id, Some("99887766".to_string()));
        assert_eq!(signal.brand_id, 42);
        assert_eq!(signal.summary, Some("Test tweet".to_string()));
        assert!(signal.title.is_none());
        assert!(signal.source_url.is_none());
        assert!(signal.published_at.is_none());
    }

    #[test]
    fn tweet_to_signal_with_datetime() {
        let dt = chrono::NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc();
        let tweet = Tweet {
            id: "42".to_string(),
            text: "Dated tweet".to_string(),
            created_at: Some(dt),
        };
        let signal = tweet_to_signal(1, tweet);
        assert_eq!(signal.published_at, Some(dt));
    }

    // -- truncate -----------------------------------------------------------

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

    #[test]
    fn truncate_multibyte_chars() {
        // Each emoji is multiple bytes but one char -- verify no panic
        let emoji_str: String = std::iter::repeat('\u{1F600}').take(2500).collect();
        let result = truncate(&emoji_str);
        assert_eq!(result.chars().count(), MAX_SUMMARY_LEN);
    }
}
