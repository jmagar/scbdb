//! Extraction strategy implementations for the store locator pipeline.

mod embed;
mod jsonld;
mod locally;
mod storemapper;

#[cfg(test)]
pub(super) use embed::extract_balanced_array;
pub(super) use embed::extract_json_embed_locations;
pub(super) use jsonld::extract_jsonld_locations;
pub(super) use locally::{extract_locally_company_id, fetch_locally_stores};
pub(super) use storemapper::{extract_storemapper_token, fetch_storemapper_stores};
