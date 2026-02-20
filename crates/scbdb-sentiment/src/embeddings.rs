//! TEI (Text Embeddings Inference) client for vector generation.

use serde::Serialize;

use crate::error::SentimentError;

/// Maximum number of texts per /embed call.
const BATCH_SIZE: usize = 64;

/// TEI HTTP client.
pub(crate) struct TeiClient {
    client: reqwest::Client,
    url: String,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    inputs: &'a [&'a str],
}

impl TeiClient {
    /// Create a new `TeiClient`.
    #[must_use]
    pub(crate) fn new(tei_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: format!("{tei_url}/embed"),
        }
    }

    /// Generate embeddings for a batch of texts.
    ///
    /// Texts are batched into groups of [`BATCH_SIZE`] (64) per request.
    /// Returns one embedding vector per input text, in the same order.
    ///
    /// # Errors
    ///
    /// Returns [`SentimentError::Tei`] if the request fails or the response
    /// cannot be parsed.
    pub(crate) async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SentimentError> {
        let mut all_embeddings = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(BATCH_SIZE) {
            let request = EmbedRequest { inputs: chunk };
            let response = self
                .client
                .post(&self.url)
                .json(&request)
                .send()
                .await
                .map_err(|e| SentimentError::Tei(format!("TEI request failed: {e}")))?;

            if !response.status().is_success() {
                return Err(SentimentError::Tei(format!(
                    "TEI returned status {}",
                    response.status()
                )));
            }

            let embeddings: Vec<Vec<f32>> = response
                .json()
                .await
                .map_err(|e| SentimentError::Tei(format!("TEI response parse error: {e}")))?;

            if embeddings.len() != chunk.len() {
                return Err(SentimentError::Tei(format!(
                    "TEI returned {} embeddings for {} inputs",
                    embeddings.len(),
                    chunk.len()
                )));
            }

            all_embeddings.extend(embeddings);
        }

        Ok(all_embeddings)
    }
}
