use thiserror::Error;

#[derive(Debug, Error)]
pub enum LegiscanError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}

/// Fetch bill JSON from the given URL.
///
/// # Errors
///
/// Returns [`LegiscanError::Http`] if the HTTP request or JSON decoding fails.
pub async fn fetch_bill_json(url: &str) -> Result<serde_json::Value, LegiscanError> {
    let response = reqwest::get(url).await?;
    let payload = response.json::<serde_json::Value>().await?;
    Ok(payload)
}
