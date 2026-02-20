use thiserror::Error;

#[derive(Debug, Error)]
pub enum SentimentError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("Reddit API error: {0}")]
    Reddit(String),

    #[error("Qdrant error: {0}")]
    Qdrant(String),

    #[error("TEI embed error: {0}")]
    Tei(String),

    #[error("normalization error: {0}")]
    Normalization(String),
}
