//! Row types for the `store_locations` table.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

/// Input record for inserting/upserting a store location.
#[derive(Debug, Clone)]
pub struct NewStoreLocation {
    pub location_key: String,
    pub name: String,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub phone: Option<String>,
    pub external_id: Option<String>,
    pub locator_source: Option<String>,
    pub raw_data: serde_json::Value,
}

/// A row from the `store_locations` table.
///
/// `raw_data` is omitted — it is write-only operational storage and not
/// needed in read-back queries.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StoreLocationRow {
    pub id: i64,
    pub public_id: Uuid,
    pub brand_id: i64,
    pub location_key: String,
    pub name: String,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: String,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub phone: Option<String>,
    pub external_id: Option<String>,
    pub locator_source: Option<String>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Per-brand store location stats for the API dashboard.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocationsDashboardRow {
    pub brand_name: String,
    pub brand_slug: String,
    /// Total active locations for this brand.
    pub active_count: i64,
    /// Locations first seen in the last 7 days.
    pub new_this_week: i64,
    /// Distinct US states covered by active locations.
    pub states_covered: i64,
    /// Most common locator source (`locally` / `storemapper` / `jsonld` / `json_embed`).
    pub locator_source: Option<String>,
    /// Timestamp of the most recent active location update.
    pub last_seen_at: Option<DateTime<Utc>>,
}

/// Per-state aggregate for the coverage tile map.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocationsByStateRow {
    pub state: String,
    pub brand_count: i64,
    pub location_count: i64,
}

/// A pin row for the map — active locations with coordinates joined with brand info.
///
/// Used to populate the interactive `MapLibre` GL JS map. Only locations where
/// both `latitude` and `longitude` are non-null are included.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocationPinRow {
    pub id: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub store_name: String,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub locator_source: Option<String>,
    pub brand_name: String,
    pub brand_slug: String,
    pub brand_relationship: String,
    pub brand_tier: i16,
}
