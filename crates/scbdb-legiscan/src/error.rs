use thiserror::Error;

/// Errors returned by the `LegiScan` API client.
#[derive(Debug, Error)]
pub enum LegiscanError {
    /// Network or TLS failure from the underlying HTTP client.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The `LegiScan` API returned `"status": "ERROR"` with a message.
    #[error("LegiScan API error: {0}")]
    ApiError(String),

    /// The response body could not be deserialized into the expected type.
    #[error("JSON deserialization error for {context}: {source}")]
    Deserialize {
        context: String,
        #[source]
        source: serde_json::Error,
    },
}
