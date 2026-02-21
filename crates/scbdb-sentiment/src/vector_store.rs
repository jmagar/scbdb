//! Qdrant vector store client for sentiment signal deduplication and storage.

use std::collections::HashMap;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::SentimentError;
use crate::types::SentimentSignal;

/// Vector dimension for Qwen3-Embedding-0.6B.
const VECTOR_DIM: u64 = 1024;

/// Qdrant HTTP client.
pub(crate) struct QdrantClient {
    client: reqwest::Client,
    base_url: String,
    collection: String,
}

#[derive(Serialize)]
struct CreateCollectionRequest {
    vectors: VectorsConfig,
}

#[derive(Serialize)]
struct VectorsConfig {
    size: u64,
    distance: String,
}

#[derive(Serialize)]
struct UpsertPointsRequest {
    points: Vec<Point>,
}

#[derive(Serialize)]
struct Point {
    id: u64,
    vector: Vec<f32>,
    payload: HashMap<String, serde_json::Value>,
}

impl QdrantClient {
    /// Create a new `QdrantClient`.
    #[must_use]
    pub(crate) fn new(qdrant_url: &str, collection: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: qdrant_url.to_string(),
            collection: collection.to_string(),
        }
    }

    /// Ensure the sentiment collection exists, creating it if absent.
    ///
    /// Uses cosine distance and 1024-dimensional vectors.
    ///
    /// # Errors
    ///
    /// Returns [`SentimentError::Qdrant`] on network or API failure.
    pub(crate) async fn ensure_collection(&self) -> Result<(), SentimentError> {
        let url = format!("{}/collections/{}", self.base_url, self.collection);
        let check = self.client.get(&url).send().await;

        // If the collection already exists, return early.
        if let Ok(resp) = check {
            if resp.status().is_success() {
                return Ok(());
            }
        }

        // Create the collection.
        let create_url = format!("{}/collections/{}", self.base_url, self.collection);
        let body = CreateCollectionRequest {
            vectors: VectorsConfig {
                size: VECTOR_DIM,
                distance: "Cosine".to_string(),
            },
        };

        let resp = self
            .client
            .put(&create_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                SentimentError::Qdrant(format!("collection create request failed: {e}"))
            })?;

        if !resp.status().is_success() {
            return Err(SentimentError::Qdrant(format!(
                "collection create returned status {}",
                resp.status()
            )));
        }

        Ok(())
    }

    /// Check if a signal (by URL) already exists in the collection.
    ///
    /// Uses the deterministic point ID derived from the URL hash.
    ///
    /// # Errors
    ///
    /// Returns [`SentimentError::Qdrant`] on network failure.
    pub(crate) async fn signal_exists(&self, url: &str) -> Result<bool, SentimentError> {
        let point_id = url_to_point_id(url);
        let get_url = format!(
            "{}/collections/{}/points/{point_id}",
            self.base_url, self.collection
        );

        let resp = self
            .client
            .get(&get_url)
            .send()
            .await
            .map_err(|e| SentimentError::Qdrant(format!("point check request failed: {e}")))?;

        Ok(resp.status().is_success())
    }

    /// Upsert a signal with its embedding into the collection.
    ///
    /// The point ID is derived from the signal URL so the same URL is
    /// always stored at the same point (natural deduplication).
    ///
    /// # Errors
    ///
    /// Returns [`SentimentError::Qdrant`] on network or API failure.
    pub(crate) async fn upsert_signal(
        &self,
        signal: &SentimentSignal,
        embedding: Vec<f32>,
    ) -> Result<(), SentimentError> {
        let point_id = url_to_point_id(&signal.url);

        let mut payload = HashMap::new();
        payload.insert(
            "brand_slug".to_string(),
            serde_json::json!(signal.brand_slug),
        );
        payload.insert("source".to_string(), serde_json::json!(signal.source));
        payload.insert("url".to_string(), serde_json::json!(signal.url));
        payload.insert("text".to_string(), serde_json::json!(signal.text));
        payload.insert("score".to_string(), serde_json::json!(signal.score));

        let body = UpsertPointsRequest {
            points: vec![Point {
                id: point_id,
                vector: embedding,
                payload,
            }],
        };

        let upsert_url = format!("{}/collections/{}/points", self.base_url, self.collection);

        let resp = self
            .client
            .put(&upsert_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| SentimentError::Qdrant(format!("upsert request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(SentimentError::Qdrant(format!(
                "upsert returned status {}",
                resp.status()
            )));
        }

        Ok(())
    }
}

/// Derive a stable Qdrant point ID (u64) from a URL.
///
/// Takes the first 8 bytes of SHA-256(url) and interprets them as a
/// big-endian u64. The same URL always produces the same ID.
pub(crate) fn url_to_point_id(url: &str) -> u64 {
    let hash = Sha256::digest(url.as_bytes());
    let bytes: [u8; 8] = hash[..8].try_into().expect("SHA256 is at least 8 bytes");
    u64::from_be_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_to_point_id_is_stable() {
        let url = "https://example.com/article-1";
        let id1 = url_to_point_id(url);
        let id2 = url_to_point_id(url);
        assert_eq!(id1, id2, "same URL must produce same point ID");
    }

    #[test]
    fn different_urls_produce_different_ids() {
        let id1 = url_to_point_id("https://example.com/a");
        let id2 = url_to_point_id("https://example.com/b");
        assert_ne!(id1, id2);
    }
}
