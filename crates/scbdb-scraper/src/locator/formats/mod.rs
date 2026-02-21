//! Extraction strategy implementations for the store locator pipeline.

mod agile_store_locator;
mod askhoodie;
mod beveragefinder;
mod destini;
mod embed;
mod jsonld;
mod locally;
mod roseperl;
mod stockist;
mod storemapper;
mod storepoint;
mod storerocket;
mod vtinfo;

pub(super) use agile_store_locator::{
    extract_agile_store_locator_config, fetch_agile_store_locator_stores,
};
pub(super) use askhoodie::{extract_askhoodie_embed_id, fetch_askhoodie_stores};
pub(super) use beveragefinder::{extract_beveragefinder_key, fetch_beveragefinder_stores};
pub(super) use destini::{discover_destini_locator_config, fetch_destini_stores};
#[cfg(test)]
pub(super) use embed::extract_balanced_array;
pub(super) use embed::extract_json_embed_locations;
pub(super) use jsonld::extract_jsonld_locations;
pub(super) use locally::{extract_locally_company_id, fetch_locally_stores};
pub(super) use roseperl::{extract_roseperl_wtb_url, fetch_roseperl_stores};
pub(super) use stockist::{extract_stockist_widget_tag, fetch_stockist_stores};
pub(super) use storemapper::{
    extract_storemapper_token, extract_storemapper_user_id, fetch_storemapper_stores,
    fetch_storemapper_stores_by_user_id,
};
pub(super) use storepoint::{extract_storepoint_widget_id, fetch_storepoint_stores};
pub(super) use storerocket::{discover_storerocket_account, fetch_storerocket_stores};
pub(super) use vtinfo::{extract_vtinfo_embed, fetch_vtinfo_stores};
