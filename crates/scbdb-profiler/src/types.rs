use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A signal collected from an external source, ready for ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedSignal {
    pub brand_id: i64,
    pub signal_type: String,
    pub source_platform: Option<String>,
    pub source_url: Option<String>,
    pub external_id: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub view_count: Option<i32>,
    pub like_count: Option<i32>,
    pub comment_count: Option<i32>,
    pub share_count: Option<i32>,
    pub published_at: Option<DateTime<Utc>>,
}

/// Result of a profiling run for a single brand.
#[derive(Debug, Clone)]
pub struct BrandProfileRunResult {
    pub brand_id: i64,
    pub signals_collected: usize,
    pub signals_embedded: usize,
    pub errors: Vec<String>,
}
