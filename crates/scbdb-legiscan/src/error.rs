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

    /// `LegiScan` returned a quota-exhaustion error (daily or monthly limit hit).
    ///
    /// This is a hard stop — further requests will fail with the same response,
    /// so retrying would only waste the remaining monthly budget. Callers must
    /// stop immediately and surface this to the operator.
    #[error("LegiScan quota exceeded: {0}")]
    QuotaExceeded(String),

    /// The per-session request budget configured via `--max-requests` was reached.
    ///
    /// No more HTTP requests will be issued for this run. Raise `--max-requests`
    /// or reduce `--max-pages` / the number of keywords if more coverage is needed.
    #[error("request budget exceeded: used {used} of {limit} allowed requests")]
    BudgetExceeded { used: u32, limit: u32 },
}
