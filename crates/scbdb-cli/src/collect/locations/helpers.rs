//! DB helpers and type conversion for the locations collection pipeline.

/// Load brands eligible for location collection.
///
/// When `brand_filter` is `Some(slug)`, returns only that brand (error if not
/// found).  When `None`, returns all active brands regardless of whether they
/// have a `store_locator_url` — auto-discovery handles the rest.
pub(super) async fn load_brands_for_locations(
    pool: &sqlx::PgPool,
    brand_filter: Option<&str>,
) -> anyhow::Result<Vec<scbdb_db::BrandRow>> {
    if let Some(slug) = brand_filter {
        let brand = scbdb_db::get_brand_by_slug(pool, slug)
            .await?
            .ok_or_else(|| anyhow::anyhow!("brand '{slug}' not found"))?;
        Ok(vec![brand])
    } else {
        Ok(scbdb_db::list_active_brands(pool).await?)
    }
}

/// Record a per-brand failure in `collection_run_brands` on a best-effort basis.
pub(super) async fn record_brand_failure(
    pool: &sqlx::PgPool,
    run_id: i64,
    brand: &scbdb_db::BrandRow,
    error_msg: &str,
) {
    if let Err(e) = scbdb_db::upsert_collection_run_brand(
        pool,
        run_id,
        brand.id,
        "failed",
        None,
        Some(error_msg),
    )
    .await
    {
        tracing::error!(
            run_id,
            brand = %brand.slug,
            error = %e,
            "failed to record brand failure in collection_run_brands"
        );
    }
}

/// Log added/removed store location counts after upsert+deactivate.
///
/// `prev` is the active key set snapshotted **before** the upsert.
/// `curr_keys` is the list of keys from the current scrape.
pub(super) fn log_location_changeset(
    brand_slug: &str,
    prev: &std::collections::HashSet<String>,
    curr_keys: &[String],
) {
    let curr: std::collections::HashSet<_> = curr_keys.iter().cloned().collect();
    let added = curr.difference(prev).count();
    let removed = prev.difference(&curr).count();
    if added > 0 {
        tracing::info!(brand = %brand_slug, count = added, "new store locations detected");
    }
    if removed > 0 {
        tracing::info!(brand = %brand_slug, count = removed, "store locations deactivated");
    }
}

/// Convert a [`scbdb_scraper::RawStoreLocation`] to a [`scbdb_db::NewStoreLocation`].
///
/// `country` defaults to `"US"` when not present in the raw record.
pub(super) fn raw_to_new_location(
    loc: &scbdb_scraper::RawStoreLocation,
    location_key: String,
) -> scbdb_db::NewStoreLocation {
    scbdb_db::NewStoreLocation {
        location_key,
        name: loc.name.clone(),
        address_line1: loc.address_line1.clone(),
        city: loc.city.clone(),
        state: loc.state.clone(),
        zip: loc.zip.clone(),
        country: loc.country.clone().or(Some("US".to_string())),
        latitude: loc.latitude,
        longitude: loc.longitude,
        phone: loc.phone.clone(),
        external_id: loc.external_id.clone(),
        locator_source: Some(loc.locator_source.clone()),
        raw_data: loc.raw_data.clone(),
    }
}
