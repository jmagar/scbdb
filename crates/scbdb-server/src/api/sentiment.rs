// These types have no handlers yet — handlers arrive in Task 5.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(super) struct SentimentSummaryItem {
    pub brand_name: String,
    pub brand_slug: String,
    pub score: Decimal,
    pub signal_count: i32,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(super) struct SentimentSnapshotItem {
    pub brand_name: String,
    pub brand_slug: String,
    pub score: Decimal,
    pub signal_count: i32,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SentimentSnapshotsQuery {
    pub limit: Option<i64>,
}
