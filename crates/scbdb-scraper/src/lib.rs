use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScraperError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}

pub async fn fetch_products_json(url: &str) -> Result<serde_json::Value, ScraperError> {
    let response = reqwest::get(url).await?;
    let payload = response.json::<serde_json::Value>().await?;
    Ok(payload)
}
