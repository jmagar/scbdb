//! Sentiment analysis pipeline for SCBDB.
//!
//! Collects brand signals from Google News RSS and Reddit, embeds them via TEI,
//! stores deduplicated vectors in Qdrant, and scores them using a domain-specific
//! lexicon. Aggregates per-brand scores for storage in `sentiment_snapshots`.

pub mod error;
pub mod pipeline;
pub mod scorer;
pub mod types;

mod embeddings;
mod sources;
mod vector_store;

pub use error::SentimentError;
pub use pipeline::run_brand_sentiment;
pub use scorer::lexicon_score;
pub use types::{BrandSentimentResult, SentimentConfig, SentimentSignal};
