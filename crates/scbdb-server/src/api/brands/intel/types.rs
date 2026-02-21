//! Response item types for the F4 brand intel endpoints.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(in crate::api) struct FundingEventItem {
    pub id: i64,
    pub event_type: String,
    pub amount_usd: Option<i64>,
    pub announced_at: Option<NaiveDate>,
    pub investors: Option<Vec<String>>,
    pub acquirer: Option<String>,
    pub source_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct LabTestItem {
    pub id: i64,
    pub product_id: Option<i64>,
    pub variant_id: Option<i64>,
    pub lab_name: Option<String>,
    pub test_date: Option<NaiveDate>,
    pub report_url: Option<String>,
    pub thc_mg_actual: Option<Decimal>,
    pub cbd_mg_actual: Option<Decimal>,
    pub total_cannabinoids_mg: Option<Decimal>,
    pub passed: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct LegalProceedingItem {
    pub id: i64,
    pub proceeding_type: String,
    pub jurisdiction: Option<String>,
    pub case_number: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub status: String,
    pub filed_at: Option<NaiveDate>,
    pub resolved_at: Option<NaiveDate>,
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct SponsorshipItem {
    pub id: i64,
    pub entity_name: String,
    pub entity_type: String,
    pub deal_type: String,
    pub announced_at: Option<NaiveDate>,
    pub ends_at: Option<NaiveDate>,
    pub source_url: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct DistributorItem {
    pub id: i64,
    pub distributor_name: String,
    pub distributor_slug: String,
    pub states: Option<Vec<String>>,
    pub territory_type: String,
    pub channel_type: String,
    pub started_at: Option<NaiveDate>,
    pub ended_at: Option<NaiveDate>,
    pub is_active: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct CompetitorItem {
    pub id: i64,
    pub brand_id: i64,
    pub competitor_brand_id: i64,
    pub relationship_type: String,
    pub distributor_name: Option<String>,
    pub states: Option<Vec<String>>,
    pub notes: Option<String>,
    pub first_observed_at: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct MediaAppearanceItem {
    pub id: i64,
    pub brand_signal_id: Option<i64>,
    pub appearance_type: String,
    pub outlet_name: String,
    pub title: Option<String>,
    pub host_or_author: Option<String>,
    pub aired_at: Option<NaiveDate>,
    pub duration_seconds: Option<i32>,
    pub source_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}
