use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScraperError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON deserialization error for {context}: {source}")]
    Deserialize {
        context: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("rate limited by {domain} (retry after {retry_after_secs}s)")]
    RateLimited {
        domain: String,
        retry_after_secs: u64,
    },

    #[error("endpoint not found: {url}")]
    NotFound { url: String },

    #[error("unexpected HTTP status {status} from {url}")]
    UnexpectedStatus { status: u16, url: String },

    #[error("normalization error for product {source_product_id}: {reason}")]
    Normalization {
        source_product_id: String,
        reason: String,
    },

    #[error("pagination limit reached for {shop_url}: exceeded {max_pages} pages")]
    PaginationLimit { shop_url: String, max_pages: usize },

    #[error("invalid shop URL \"{shop_url}\": {reason}")]
    InvalidShopUrl { shop_url: String, reason: String },
}
