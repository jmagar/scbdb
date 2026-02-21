//! TEI embeddings client and Qdrant point-ID derivation.

use crate::error::ProfilerError;
use reqwest::Client;
use sha2::{Digest, Sha256};

/// TEI embedding request body.
#[derive(serde::Serialize)]
struct EmbedRequest<'a> {
    inputs: &'a str,
}

/// Derive a deterministic UUID-formatted Qdrant point ID from signal content.
///
/// Uses SHA-256 of the input string, taking the first 16 bytes and formatting
/// as a UUID-style hex string (8-4-4-4-12).
#[must_use]
pub fn signal_point_id(content: &str) -> String {
    let hash = Sha256::digest(content.as_bytes());
    let b = &hash[..16];
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3],
        b[4], b[5],
        b[6], b[7],
        b[8], b[9],
        b[10], b[11], b[12], b[13], b[14], b[15],
    )
}

/// Fetch embeddings from TEI for the given text.
///
/// Sends a single text to the `/embed` endpoint and returns the first
/// embedding vector from the response.
///
/// # Errors
///
/// Returns [`ProfilerError::Http`] on network or HTTP failures, or
/// [`ProfilerError::Other`] if TEI returns an empty response.
pub async fn embed_text(
    client: &Client,
    tei_url: &str,
    text: &str,
) -> Result<Vec<f32>, ProfilerError> {
    let url = format!("{}/embed", tei_url.trim_end_matches('/'));
    let body = EmbedRequest { inputs: text };
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    let embeddings: Vec<Vec<f32>> = response.json().await?;
    embeddings
        .into_iter()
        .next()
        .ok_or_else(|| ProfilerError::Other("TEI returned empty embedding".into()))
}

/// Embed a signal and return its deterministic point ID alongside the vector.
///
/// Combines [`signal_point_id`] (for deduplication) with [`embed_text`]
/// (for vector generation). The `content_key` is the string used to derive
/// the stable point ID (typically the signal's source URL or unique key).
///
/// # Errors
///
/// Propagates any errors from [`embed_text`].
pub async fn embed_signal(
    client: &Client,
    tei_url: &str,
    text: &str,
    content_key: &str,
) -> Result<(String, Vec<f32>), ProfilerError> {
    let point_id = signal_point_id(content_key);
    let embedding = embed_text(client, tei_url, text).await?;
    Ok((point_id, embedding))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_point_id_is_deterministic() {
        let id1 = signal_point_id("test content");
        let id2 = signal_point_id("test content");
        assert_eq!(id1, id2);
    }

    #[test]
    fn signal_point_id_differs_for_different_content() {
        let id1 = signal_point_id("content A");
        let id2 = signal_point_id("content B");
        assert_ne!(id1, id2);
    }

    #[test]
    fn signal_point_id_is_valid_uuid_format() {
        let id = signal_point_id("test");
        // UUID format: 8-4-4-4-12 hex chars = 36 total with dashes
        assert_eq!(id.len(), 36);
        assert_eq!(&id[8..9], "-");
        assert_eq!(&id[13..14], "-");
        assert_eq!(&id[18..19], "-");
        assert_eq!(&id[23..24], "-");
    }
}
