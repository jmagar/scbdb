//! Brand intelligence API handlers (F1-F4).
//!
//! - `GET /api/v1/brands`               — brand list with completeness scores
//! - `GET /api/v1/brands/:slug`          — full brand profile
//! - `GET /api/v1/brands/:slug/signals`  — cursor-paginated signal feed
//! - `GET /api/v1/brands/:slug/funding`  — funding events
//! - `GET /api/v1/brands/:slug/lab-tests`— lab test results
//! - `GET /api/v1/brands/:slug/legal`    — legal proceedings
//! - `GET /api/v1/brands/:slug/sponsorships` — sponsorships
//! - `GET /api/v1/brands/:slug/distributors` — distributors
//! - `GET /api/v1/brands/:slug/competitors`  — competitor relationships
//! - `GET /api/v1/brands/:slug/media`    — media appearances

mod detail;
mod intel;
mod list;
mod signals;

pub(super) use detail::get_brand;
pub(super) use intel::{
    list_competitors, list_distributors, list_funding, list_lab_tests, list_legal, list_media,
    list_sponsorships,
};
pub(super) use list::list_brands;
pub(super) use signals::list_brand_signals;

use super::{map_db_error, ApiError};

/// Resolve a brand slug to a `BrandRow`, returning 404 if not found.
async fn resolve_brand(
    pool: &sqlx::PgPool,
    slug: &str,
    request_id: &str,
) -> Result<scbdb_db::BrandRow, ApiError> {
    scbdb_db::get_brand_by_slug(pool, slug)
        .await
        .map_err(|e| map_db_error(request_id.to_owned(), &e))?
        .ok_or_else(|| ApiError::new(request_id, "not_found", format!("brand '{slug}' not found")))
}
