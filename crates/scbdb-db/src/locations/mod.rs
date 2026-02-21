//! Database operations for the `store_locations` table.

mod read;
mod types;
mod write;

pub use read::{
    get_active_location_keys_for_brand, list_active_location_pins, list_active_locations_by_brand,
    list_locations_by_state, list_locations_dashboard_summary, list_new_locations_since,
};
pub use types::{
    LocationPinRow, LocationsByStateRow, LocationsDashboardRow, NewStoreLocation, StoreLocationRow,
};
pub use write::{deactivate_missing_locations, upsert_store_locations};
