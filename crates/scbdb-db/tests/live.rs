//! Live integration tests for scbdb-db using `#[sqlx::test]`.
//!
//! Each test gets a fresh, fully-migrated Postgres database spun up by the
//! sqlx test harness. The `migrations` path is relative to the crate root
//! (`crates/scbdb-db/`), so `"../../migrations"` resolves to the workspace
//! migration directory.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use scbdb_core::{NormalizedProduct, NormalizedVariant};
use scbdb_db::{
    complete_collection_run, create_collection_run, deactivate_missing_locations,
    fail_collection_run, get_bill_by_jurisdiction_number, get_brand_by_slug, get_collection_run,
    get_last_price_snapshot, insert_brand_funding_event, insert_brand_lab_test,
    insert_brand_legal_proceeding, insert_brand_sponsorship, insert_price_snapshot_if_changed,
    list_active_brands, list_bill_events, list_bills, list_brand_funding_events,
    list_brand_lab_tests, list_brand_legal_proceedings, list_brand_sponsorships,
    list_brands_without_profiles, list_collection_run_brands, list_locations_by_state,
    list_locations_dashboard_summary, start_collection_run, update_brand_logo, upsert_bill,
    upsert_bill_event, upsert_brand_profile, upsert_collection_run_brand, upsert_product,
    upsert_store_locations, upsert_variant, NewBrandFundingEvent, NewBrandLabTest,
    NewBrandLegalProceeding, NewBrandSponsorship, NewStoreLocation,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Insert a minimal brand row and return its generated `id`.
async fn insert_test_brand(pool: &sqlx::PgPool, slug: &str, is_active: bool) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO brands (name, slug, relationship, tier, shop_url, is_active) \
         VALUES ($1, $2, 'portfolio', 1, $3, $4) RETURNING id",
    )
    .bind(format!("Test Brand {slug}"))
    .bind(slug)
    .bind(format!("https://{slug}.com"))
    .bind(is_active)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("insert_test_brand failed for slug '{slug}': {e}"))
}

fn make_normalized_product(source_product_id: &str) -> NormalizedProduct {
    NormalizedProduct {
        source_product_id: source_product_id.to_string(),
        source_platform: "shopify".to_string(),
        name: "Test Product".to_string(),
        description: None,
        product_type: None,
        tags: vec![],
        handle: Some("test-product".to_string()),
        status: "active".to_string(),
        source_url: None,
        vendor: None,
        primary_image_url: None,
        image_gallery: vec![],
        variants: vec![make_normalized_variant("VAR-001")],
    }
}

fn make_normalized_variant(source_variant_id: &str) -> NormalizedVariant {
    NormalizedVariant {
        source_variant_id: source_variant_id.to_string(),
        sku: None,
        title: "Default Title".to_string(),
        price: "12.99".to_string(),
        compare_at_price: None,
        currency_code: "USD".to_string(),
        source_url: None,
        is_available: true,
        is_default: true,
        dosage_mg: Some(5.0),
        cbd_mg: None,
        size_value: Some(12.0),
        size_unit: Some("oz".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Section 1: Collection Run Lifecycle
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn collection_run_lifecycle_queued_to_succeeded(pool: sqlx::PgPool) {
    let run = create_collection_run(&pool, "products", "cli")
        .await
        .expect("create_collection_run failed");

    assert_eq!(run.status, "queued");
    assert!(run.started_at.is_none());
    assert!(run.completed_at.is_none());
    assert_eq!(run.records_processed, 0);

    start_collection_run(&pool, run.id)
        .await
        .expect("start_collection_run failed");

    complete_collection_run(&pool, run.id, 5)
        .await
        .expect("complete_collection_run failed");

    let fetched = get_collection_run(&pool, run.id)
        .await
        .expect("get_collection_run failed");

    assert_eq!(fetched.status, "succeeded");
    assert!(fetched.started_at.is_some(), "started_at should be set");
    assert!(fetched.completed_at.is_some(), "completed_at should be set");
    assert_eq!(fetched.records_processed, 5);
    assert!(fetched.error_message.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn collection_run_lifecycle_queued_to_failed(pool: sqlx::PgPool) {
    let run = create_collection_run(&pool, "products", "cli")
        .await
        .expect("create_collection_run failed");

    start_collection_run(&pool, run.id)
        .await
        .expect("start_collection_run failed");

    fail_collection_run(&pool, run.id, "network error")
        .await
        .expect("fail_collection_run failed");

    let fetched = get_collection_run(&pool, run.id)
        .await
        .expect("get_collection_run failed");

    assert_eq!(fetched.status, "failed");
    assert!(fetched.started_at.is_some(), "started_at should be set");
    assert_eq!(fetched.error_message.as_deref(), Some("network error"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn collection_run_cannot_complete_directly_from_queued(pool: sqlx::PgPool) {
    let run = create_collection_run(&pool, "products", "cli")
        .await
        .expect("create_collection_run failed");

    let err = complete_collection_run(&pool, run.id, 1)
        .await
        .expect_err("completing a queued run should fail");

    assert!(matches!(
        err,
        scbdb_db::DbError::InvalidCollectionRunTransition {
            expected_status: "running",
            ..
        }
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn collection_run_start_fails_for_unknown_id(pool: sqlx::PgPool) {
    let err = start_collection_run(&pool, 999_999)
        .await
        .expect_err("starting an unknown run should fail");
    assert!(matches!(
        err,
        scbdb_db::DbError::InvalidCollectionRunTransition {
            expected_status: "queued",
            ..
        }
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn collection_run_cannot_fail_directly_from_queued(pool: sqlx::PgPool) {
    let run = create_collection_run(&pool, "products", "cli")
        .await
        .expect("create failed");

    let err = fail_collection_run(&pool, run.id, "test error")
        .await
        .expect_err("expected error when failing a queued run");

    assert!(
        matches!(
            err,
            scbdb_db::DbError::InvalidCollectionRunTransition { .. }
        ),
        "expected InvalidCollectionRunTransition, got {err:?}"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn collection_run_failed_sets_completed_at(pool: sqlx::PgPool) {
    let run = create_collection_run(&pool, "products", "cli")
        .await
        .expect("create failed");
    start_collection_run(&pool, run.id)
        .await
        .expect("start failed");
    fail_collection_run(&pool, run.id, "test failure")
        .await
        .expect("fail failed");

    let fetched = get_collection_run(&pool, run.id).await.expect("get failed");

    assert_eq!(fetched.status, "failed");
    assert!(
        fetched.completed_at.is_some(),
        "completed_at should be set after fail"
    );
}

// ---------------------------------------------------------------------------
// Section 2: Product Upsert Idempotency
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn product_upsert_is_idempotent(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "cann", true).await;
    let product = make_normalized_product("PROD-001");

    let id_first = upsert_product(&pool, brand_id, &product)
        .await
        .expect("first upsert_product failed");

    let id_second = upsert_product(&pool, brand_id, &product)
        .await
        .expect("second upsert_product failed");

    assert_eq!(
        id_first, id_second,
        "upsert must return the same id both times"
    );

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM products WHERE brand_id = $1 AND source_product_id = $2",
    )
    .bind(brand_id)
    .bind("PROD-001")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count, 1,
        "exactly one product row should exist after two upserts"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_upsert_persists_vendor_field(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "cann-vendor", true).await;
    let mut product = make_normalized_product("PROD-VENDOR-001");
    product.vendor = Some("CANN".to_string());

    upsert_product(&pool, brand_id, &product)
        .await
        .expect("first upsert_product failed");

    let vendor: Option<String> = sqlx::query_scalar(
        "SELECT vendor FROM products WHERE brand_id = $1 AND source_product_id = $2",
    )
    .bind(brand_id)
    .bind("PROD-VENDOR-001")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        vendor.as_deref(),
        Some("CANN"),
        "vendor should be persisted"
    );

    // Verify vendor is updated on conflict (not ignored).
    product.vendor = Some("CANN Beverages".to_string());
    upsert_product(&pool, brand_id, &product)
        .await
        .expect("second upsert_product failed");

    let updated_vendor: Option<String> = sqlx::query_scalar(
        "SELECT vendor FROM products WHERE brand_id = $1 AND source_product_id = $2",
    )
    .bind(brand_id)
    .bind("PROD-VENDOR-001")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        updated_vendor.as_deref(),
        Some("CANN Beverages"),
        "vendor should be overwritten on conflict"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_upsert_updates_name_on_conflict(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "cann-2", true).await;

    let mut product = make_normalized_product("PROD-002");
    upsert_product(&pool, brand_id, &product)
        .await
        .expect("first upsert failed");

    product.name = "Updated Product Name".to_string();
    upsert_product(&pool, brand_id, &product)
        .await
        .expect("second upsert failed");

    let name: String = sqlx::query_scalar(
        "SELECT name FROM products WHERE brand_id = $1 AND source_product_id = $2",
    )
    .bind(brand_id)
    .bind("PROD-002")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(name, "Updated Product Name");
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_upsert_persists_additional_fields(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "cann-extra", true).await;
    let mut product = make_normalized_product("PROD-EXTRA-001");
    product.description = Some("A rich description".to_string());
    product.product_type = Some("Beverage".to_string());
    product.tags = vec!["sparkling".to_string(), "5mg".to_string()];
    product.handle = Some("prod-extra-001".to_string());
    product.source_url = Some("https://example.com/products/prod-extra-001".to_string());
    product.primary_image_url = Some("https://cdn.shopify.com/primary.jpg".to_string());
    product.image_gallery = vec![
        scbdb_core::NormalizedImage {
            source_image_id: Some("1001".to_string()),
            src: "https://cdn.shopify.com/primary.jpg".to_string(),
            alt: Some("Front can".to_string()),
            position: Some(1),
            width: Some(1200),
            height: Some(1200),
            variant_source_ids: vec!["VAR-001".to_string()],
        },
        scbdb_core::NormalizedImage {
            source_image_id: Some("1002".to_string()),
            src: "https://cdn.shopify.com/lifestyle.jpg".to_string(),
            alt: None,
            position: Some(2),
            width: Some(1400),
            height: Some(1200),
            variant_source_ids: vec![],
        },
    ];

    upsert_product(&pool, brand_id, &product)
        .await
        .expect("upsert_product failed");

    let row = sqlx::query_as::<
        _,
        (
            Option<String>,
            Option<String>,
            Option<Vec<String>>,
            Option<String>,
            Option<String>,
            serde_json::Value,
        ),
    >(
        "SELECT description, product_type, tags, handle, source_url, metadata \
         FROM products WHERE brand_id = $1 AND source_product_id = $2",
    )
    .bind(brand_id)
    .bind("PROD-EXTRA-001")
    .fetch_one(&pool)
    .await
    .expect("fetch product row failed");

    assert_eq!(row.0.as_deref(), Some("A rich description"));
    assert_eq!(row.1.as_deref(), Some("Beverage"));
    assert_eq!(
        row.2,
        Some(vec!["sparkling".to_string(), "5mg".to_string()])
    );
    assert_eq!(row.3.as_deref(), Some("prod-extra-001"));
    assert_eq!(
        row.4.as_deref(),
        Some("https://example.com/products/prod-extra-001")
    );
    assert_eq!(
        row.5["primary_image_url"].as_str(),
        Some("https://cdn.shopify.com/primary.jpg")
    );
    assert_eq!(
        row.5["image_gallery"]
            .as_array()
            .map_or(0, std::vec::Vec::len),
        2
    );

    product.primary_image_url = Some("https://cdn.shopify.com/updated.jpg".to_string());
    product.image_gallery = vec![scbdb_core::NormalizedImage {
        source_image_id: Some("2001".to_string()),
        src: "https://cdn.shopify.com/updated.jpg".to_string(),
        alt: Some("Updated hero".to_string()),
        position: Some(1),
        width: Some(1000),
        height: Some(1000),
        variant_source_ids: vec![],
    }];

    upsert_product(&pool, brand_id, &product)
        .await
        .expect("upsert_product update failed");

    let updated_metadata: serde_json::Value = sqlx::query_scalar(
        "SELECT metadata FROM products WHERE brand_id = $1 AND source_product_id = $2",
    )
    .bind(brand_id)
    .bind("PROD-EXTRA-001")
    .fetch_one(&pool)
    .await
    .expect("fetch updated metadata failed");

    assert_eq!(
        updated_metadata["primary_image_url"].as_str(),
        Some("https://cdn.shopify.com/updated.jpg")
    );
}

// ---------------------------------------------------------------------------
// Section 3: Variant Upsert
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn variant_upsert_creates_and_updates(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "hiboy", true).await;
    let product = make_normalized_product("PROD-003");
    let product_id = upsert_product(&pool, brand_id, &product)
        .await
        .expect("upsert_product failed");

    let mut variant = make_normalized_variant("VAR-002");
    variant.dosage_mg = Some(5.0);

    let variant_id_first = upsert_variant(&pool, product_id, &variant)
        .await
        .expect("first upsert_variant failed");

    variant.dosage_mg = Some(10.0);
    let variant_id_second = upsert_variant(&pool, product_id, &variant)
        .await
        .expect("second upsert_variant failed");

    assert_eq!(
        variant_id_first, variant_id_second,
        "variant id must be stable across upserts"
    );

    let dosage: rust_decimal::Decimal =
        sqlx::query_scalar("SELECT dosage_mg FROM product_variants WHERE id = $1")
            .bind(variant_id_first)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        dosage,
        rust_decimal::Decimal::new(1000, 2),
        "dosage_mg should be updated to 10.00"
    );

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM product_variants WHERE product_id = $1 AND source_variant_id = $2",
    )
    .bind(product_id)
    .bind("VAR-002")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count, 1,
        "exactly one variant row should exist after two upserts"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn variant_upsert_updates_is_default_on_conflict(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "hiboy-default", true).await;
    let product = make_normalized_product("PROD-DEFAULT-001");
    let product_id = upsert_product(&pool, brand_id, &product).await.unwrap();

    let mut variant = make_normalized_variant("VAR-DEFAULT-001");
    variant.is_default = false;
    let variant_id = upsert_variant(&pool, product_id, &variant).await.unwrap();

    variant.is_default = true;
    upsert_variant(&pool, product_id, &variant).await.unwrap();

    let is_default: bool =
        sqlx::query_scalar("SELECT is_default FROM product_variants WHERE id = $1")
            .bind(variant_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(is_default);
}

// ---------------------------------------------------------------------------
// Section 4: Price Snapshot Dedup
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn price_snapshot_not_inserted_when_price_unchanged(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "cann-snap", true).await;
    let product = make_normalized_product("PROD-SNAP-001");
    let product_id = upsert_product(&pool, brand_id, &product).await.unwrap();
    let variant = make_normalized_variant("VAR-SNAP-001");
    let variant_id = upsert_variant(&pool, product_id, &variant).await.unwrap();
    let run = create_collection_run(&pool, "pricing", "cli")
        .await
        .unwrap();

    let inserted_first = insert_price_snapshot_if_changed(
        &pool,
        variant_id,
        Some(run.id),
        "12.99",
        None,
        "USD",
        None,
    )
    .await
    .expect("first insert failed");
    assert!(inserted_first, "first snapshot should be inserted");

    let inserted_second = insert_price_snapshot_if_changed(
        &pool,
        variant_id,
        Some(run.id),
        "12.99",
        None,
        "USD",
        None,
    )
    .await
    .expect("second insert failed");
    assert!(
        !inserted_second,
        "same price should NOT insert a second snapshot"
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM price_snapshots WHERE variant_id = $1")
            .bind(variant_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        count, 1,
        "only one snapshot should exist after two same-price inserts"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn price_snapshot_inserted_when_price_changes(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "cann-snap2", true).await;
    let product = make_normalized_product("PROD-SNAP-002");
    let product_id = upsert_product(&pool, brand_id, &product).await.unwrap();
    let variant = make_normalized_variant("VAR-SNAP-002");
    let variant_id = upsert_variant(&pool, product_id, &variant).await.unwrap();
    let run = create_collection_run(&pool, "pricing", "cli")
        .await
        .unwrap();

    let inserted_first = insert_price_snapshot_if_changed(
        &pool,
        variant_id,
        Some(run.id),
        "12.99",
        None,
        "USD",
        None,
    )
    .await
    .unwrap();
    assert!(inserted_first);

    let inserted_second = insert_price_snapshot_if_changed(
        &pool,
        variant_id,
        Some(run.id),
        "14.99",
        None,
        "USD",
        None,
    )
    .await
    .unwrap();
    assert!(
        inserted_second,
        "changed price SHOULD insert a new snapshot"
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM price_snapshots WHERE variant_id = $1")
            .bind(variant_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        count, 2,
        "two snapshots should exist after two different-price inserts"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn price_snapshot_inserted_when_compare_at_price_changes(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "cann-sale", true).await;
    let product = make_normalized_product("PROD-SALE-001");
    let product_id = upsert_product(&pool, brand_id, &product).await.unwrap();
    let variant = make_normalized_variant("VAR-SALE-001");
    let variant_id = upsert_variant(&pool, product_id, &variant).await.unwrap();
    let run = create_collection_run(&pool, "pricing", "cli")
        .await
        .unwrap();

    let inserted_first = insert_price_snapshot_if_changed(
        &pool,
        variant_id,
        Some(run.id),
        "12.99",
        Some("14.99"),
        "USD",
        None,
    )
    .await
    .unwrap();
    assert!(inserted_first);

    let inserted_second = insert_price_snapshot_if_changed(
        &pool,
        variant_id,
        Some(run.id),
        "12.99",
        None,
        "USD",
        None,
    )
    .await
    .unwrap();
    assert!(
        inserted_second,
        "changing compare_at_price should write a new snapshot"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn price_snapshot_allows_manual_capture_without_collection_run(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "manual-snap", true).await;
    let product = make_normalized_product("PROD-MANUAL-001");
    let product_id = upsert_product(&pool, brand_id, &product).await.unwrap();
    let variant = make_normalized_variant("VAR-MANUAL-001");
    let variant_id = upsert_variant(&pool, product_id, &variant).await.unwrap();

    let inserted =
        insert_price_snapshot_if_changed(&pool, variant_id, None, "12.99", None, "USD", None)
            .await
            .expect("manual insert failed");
    assert!(inserted);

    let stored_run_id: Option<i64> =
        sqlx::query_scalar("SELECT collection_run_id FROM price_snapshots WHERE variant_id = $1")
            .bind(variant_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(stored_run_id.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn price_snapshot_get_last_is_deterministic_for_same_timestamp(pool: sqlx::PgPool) {
    // Setup: create brand, product, variant
    let brand_id = insert_test_brand(&pool, "cann-sort", true).await;
    let product = make_normalized_product("PROD-SORT-001");
    let product_id = upsert_product(&pool, brand_id, &product).await.unwrap();
    let variant = make_normalized_variant("VAR-SORT-001");
    let variant_id = upsert_variant(&pool, product_id, &variant).await.unwrap();

    // Insert two rows with the same captured_at
    let fixed_ts = "2026-01-01 00:00:00+00";
    let id1: i64 = sqlx::query_scalar(
        "INSERT INTO price_snapshots (variant_id, captured_at, currency_code, price) \
         VALUES ($1, $2::timestamptz, 'USD', '5.00') RETURNING id",
    )
    .bind(variant_id)
    .bind(fixed_ts)
    .fetch_one(&pool)
    .await
    .expect("insert 1 failed");

    let id2: i64 = sqlx::query_scalar(
        "INSERT INTO price_snapshots (variant_id, captured_at, currency_code, price) \
         VALUES ($1, $2::timestamptz, 'USD', '6.00') RETURNING id",
    )
    .bind(variant_id)
    .bind(fixed_ts)
    .fetch_one(&pool)
    .await
    .expect("insert 2 failed");

    assert!(id2 > id1, "second insert should have higher id");

    let last = get_last_price_snapshot(&pool, variant_id)
        .await
        .expect("query failed")
        .expect("no snapshot found");

    assert_eq!(
        last.id, id2,
        "should return the higher-id row when timestamps match"
    );
}

// ---------------------------------------------------------------------------
// Section 5: Brands Queries
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn list_active_brands_returns_only_active_brands(pool: sqlx::PgPool) {
    insert_test_brand(&pool, "active-1", true).await;
    insert_test_brand(&pool, "active-2", true).await;
    insert_test_brand(&pool, "inactive-1", false).await;

    let brands = list_active_brands(&pool)
        .await
        .expect("list_active_brands failed");

    assert_eq!(brands.len(), 2, "should return exactly 2 active brands");
    assert!(
        brands.iter().all(|b| b.is_active),
        "all returned brands must have is_active=true"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_active_brands_excludes_soft_deleted_rows(pool: sqlx::PgPool) {
    insert_test_brand(&pool, "active-soft-1", true).await;
    let deleted_id = insert_test_brand(&pool, "active-soft-2", true).await;
    sqlx::query("UPDATE brands SET deleted_at = NOW() WHERE id = $1")
        .bind(deleted_id)
        .execute(&pool)
        .await
        .unwrap();

    let brands = list_active_brands(&pool).await.unwrap();
    assert!(brands.iter().all(|b| b.deleted_at.is_none()));
    assert!(!brands.iter().any(|b| b.id == deleted_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_brand_by_slug_returns_brand_when_found(pool: sqlx::PgPool) {
    insert_test_brand(&pool, "cann", true).await;

    let result = get_brand_by_slug(&pool, "cann")
        .await
        .expect("get_brand_by_slug failed");

    let brand = result.expect("expected Some(brand), got None");
    assert_eq!(brand.slug, "cann");
    assert!(brand.is_active);
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_brand_by_slug_returns_none_when_not_found(pool: sqlx::PgPool) {
    let result = get_brand_by_slug(&pool, "nonexistent")
        .await
        .expect("get_brand_by_slug failed");

    assert!(result.is_none(), "expected None for unknown slug");
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_brand_by_slug_returns_none_when_inactive(pool: sqlx::PgPool) {
    insert_test_brand(&pool, "inactive-slug", false).await;
    let result = get_brand_by_slug(&pool, "inactive-slug")
        .await
        .expect("get_brand_by_slug failed");
    assert!(result.is_none(), "expected None for inactive brand");
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_brand_logo_persists_logo_url(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "logo-brand", true).await;
    update_brand_logo(&pool, brand_id, "https://cdn.example.com/logo.png")
        .await
        .expect("update_brand_logo failed");

    let logo_url: Option<String> = sqlx::query_scalar("SELECT logo_url FROM brands WHERE id = $1")
        .bind(brand_id)
        .fetch_one(&pool)
        .await
        .expect("fetch logo_url failed");
    assert_eq!(
        logo_url.as_deref(),
        Some("https://cdn.example.com/logo.png")
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_brand_by_slug_excludes_soft_deleted(pool: sqlx::PgPool) {
    // Insert an active brand, then soft-delete it by setting deleted_at.
    let slug = "soft-deleted-brand";
    insert_test_brand(&pool, slug, true).await;
    sqlx::query("UPDATE brands SET deleted_at = NOW() WHERE slug = $1")
        .bind(slug)
        .execute(&pool)
        .await
        .expect("failed to soft-delete brand");

    let result = get_brand_by_slug(&pool, slug)
        .await
        .expect("get_brand_by_slug failed");
    assert!(
        result.is_none(),
        "expected None for soft-deleted brand, got Some"
    );
}

// ---------------------------------------------------------------------------
// Section 6: Bills and Bill Events
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn bill_upsert_is_idempotent(pool: sqlx::PgPool) {
    let id_first = upsert_bill(
        &pool,
        "SC",
        "H-1234",
        "Hemp Beverage Act",
        None,
        "introduced",
        None,
        None,
        None,
        Some("2025-2026"),
        None,
    )
    .await
    .expect("first upsert_bill failed");

    let id_second = upsert_bill(
        &pool,
        "SC",
        "H-1234",
        "Hemp Beverage Act",
        None,
        "introduced",
        None,
        None,
        None,
        Some("2025-2026"),
        None,
    )
    .await
    .expect("second upsert_bill failed");

    assert_eq!(
        id_first, id_second,
        "upsert must return the same id both times"
    );

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bills WHERE jurisdiction = 'SC' AND bill_number = 'H-1234'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count, 1,
        "exactly one bill row should exist after two upserts"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn bill_upsert_updates_status_on_conflict(pool: sqlx::PgPool) {
    upsert_bill(
        &pool,
        "SC",
        "H-2000",
        "Test Bill",
        None,
        "introduced",
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("first upsert failed");

    upsert_bill(
        &pool,
        "SC",
        "H-2000",
        "Test Bill",
        None,
        "passed",
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("second upsert failed");

    let status: String = sqlx::query_scalar(
        "SELECT status FROM bills WHERE jurisdiction = 'SC' AND bill_number = 'H-2000'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(status, "passed", "status should be updated on conflict");
}

#[sqlx::test(migrations = "../../migrations")]
async fn bill_upsert_does_not_overwrite_introduced_date(pool: sqlx::PgPool) {
    let original_date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

    upsert_bill(
        &pool,
        "SC",
        "H-3000",
        "Intro Date Bill",
        None,
        "introduced",
        None,
        Some(original_date),
        None,
        None,
        None,
    )
    .await
    .expect("first upsert failed");

    let different_date = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
    upsert_bill(
        &pool,
        "SC",
        "H-3000",
        "Intro Date Bill",
        None,
        "passed",
        None,
        Some(different_date),
        None,
        None,
        None,
    )
    .await
    .expect("second upsert failed");

    let stored_date: NaiveDate = sqlx::query_scalar(
        "SELECT introduced_date FROM bills WHERE jurisdiction = 'SC' AND bill_number = 'H-3000'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        stored_date, original_date,
        "introduced_date should be preserved from the first insert"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn bill_event_not_duplicated_on_reingest(pool: sqlx::PgPool) {
    let bill_id = upsert_bill(
        &pool,
        "SC",
        "H-4000",
        "Event Dedup Bill",
        None,
        "introduced",
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let event_date = NaiveDate::from_ymd_opt(2025, 3, 1);

    upsert_bill_event(
        &pool,
        bill_id,
        event_date,
        Some("hearing"),
        Some("house"),
        "First reading",
        None,
    )
    .await
    .expect("first event insert failed");

    upsert_bill_event(
        &pool,
        bill_id,
        event_date,
        Some("hearing"),
        Some("house"),
        "First reading",
        None,
    )
    .await
    .expect("second event insert failed");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bill_events WHERE bill_id = $1")
        .bind(bill_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count, 1, "duplicate event should not create a second row");
}

#[sqlx::test(migrations = "../../migrations")]
async fn bill_event_different_description_creates_new_row(pool: sqlx::PgPool) {
    let bill_id = upsert_bill(
        &pool,
        "SC",
        "H-4100",
        "Multi Event Bill",
        None,
        "introduced",
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let event_date = NaiveDate::from_ymd_opt(2025, 3, 1);

    upsert_bill_event(
        &pool,
        bill_id,
        event_date,
        Some("hearing"),
        Some("house"),
        "First reading",
        None,
    )
    .await
    .unwrap();

    upsert_bill_event(
        &pool,
        bill_id,
        event_date,
        Some("vote"),
        Some("house"),
        "Passed committee",
        None,
    )
    .await
    .unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bill_events WHERE bill_id = $1")
        .bind(bill_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(
        count, 2,
        "different descriptions should create distinct rows"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_bills_filters_by_jurisdiction(pool: sqlx::PgPool) {
    upsert_bill(
        &pool,
        "SC",
        "H-5000",
        "SC Bill",
        None,
        "introduced",
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    upsert_bill(
        &pool,
        "NC",
        "S-100",
        "NC Bill",
        None,
        "introduced",
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let sc_bills = list_bills(&pool, Some("SC"), 100).await.unwrap();
    assert_eq!(sc_bills.len(), 1, "should return only SC bills");
    assert_eq!(sc_bills[0].jurisdiction, "SC");
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_bills_returns_all_when_no_filter(pool: sqlx::PgPool) {
    upsert_bill(
        &pool,
        "SC",
        "H-6000",
        "SC Bill 2",
        None,
        "introduced",
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    upsert_bill(
        &pool,
        "NC",
        "S-200",
        "NC Bill 2",
        None,
        "introduced",
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let all_bills = list_bills(&pool, None, 100).await.unwrap();
    assert_eq!(
        all_bills.len(),
        2,
        "should return all bills when jurisdiction is None"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_bill_events_ordered_by_date_desc(pool: sqlx::PgPool) {
    let bill_id = upsert_bill(
        &pool,
        "SC",
        "H-7000",
        "Ordering Bill",
        None,
        "introduced",
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let date_jan = NaiveDate::from_ymd_opt(2025, 1, 10);
    let date_mar = NaiveDate::from_ymd_opt(2025, 3, 15);
    let date_feb = NaiveDate::from_ymd_opt(2025, 2, 20);

    upsert_bill_event(&pool, bill_id, date_jan, None, None, "January event", None)
        .await
        .unwrap();
    upsert_bill_event(&pool, bill_id, date_mar, None, None, "March event", None)
        .await
        .unwrap();
    upsert_bill_event(&pool, bill_id, date_feb, None, None, "February event", None)
        .await
        .unwrap();

    let events = list_bill_events(&pool, bill_id).await.unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].description, "March event");
    assert_eq!(events[1].description, "February event");
    assert_eq!(events[2].description, "January event");
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_bill_by_jurisdiction_number_found(pool: sqlx::PgPool) {
    upsert_bill(
        &pool,
        "GA",
        "H-100",
        "Georgia Hemp Act",
        Some("Regulates hemp beverages"),
        "introduced",
        None,
        None,
        None,
        Some("2025-2026"),
        None,
    )
    .await
    .unwrap();

    let bill = get_bill_by_jurisdiction_number(&pool, "GA", "H-100")
        .await
        .expect("query failed")
        .expect("expected Some(bill), got None");

    assert_eq!(bill.jurisdiction, "GA");
    assert_eq!(bill.bill_number, "H-100");
    assert_eq!(bill.title, "Georgia Hemp Act");
    assert_eq!(bill.summary.as_deref(), Some("Regulates hemp beverages"));
    assert_eq!(bill.session.as_deref(), Some("2025-2026"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_bill_by_jurisdiction_number_not_found(pool: sqlx::PgPool) {
    let result = get_bill_by_jurisdiction_number(&pool, "ZZ", "X-9999")
        .await
        .expect("query failed");

    assert!(result.is_none(), "expected None for nonexistent bill");
}

// ---------------------------------------------------------------------------
// Section 7: Collection Run Brands
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn upsert_collection_run_brand_overwrites_on_conflict(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "crb-upsert", true).await;
    let run = create_collection_run(&pool, "products", "cli")
        .await
        .expect("create_collection_run failed");

    // First call: simulate a failure recording
    upsert_collection_run_brand(&pool, run.id, brand_id, "failed", None, Some("first error"))
        .await
        .expect("first upsert_collection_run_brand failed");

    // Second call: simulate a re-run that succeeded
    upsert_collection_run_brand(&pool, run.id, brand_id, "succeeded", Some(5), None)
        .await
        .expect("second upsert_collection_run_brand failed");

    // Verify exactly one row exists for this (run_id, brand_id) pair
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM collection_run_brands \
         WHERE collection_run_id = $1 AND brand_id = $2",
    )
    .bind(run.id)
    .bind(brand_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count, 1,
        "upsert should produce exactly one row, not a duplicate"
    );

    // Verify the row reflects the second (overwriting) call
    let (status, records_processed, error_message): (String, i32, Option<String>) = sqlx::query_as(
        "SELECT status, records_processed, error_message \
             FROM collection_run_brands \
             WHERE collection_run_id = $1 AND brand_id = $2",
    )
    .bind(run.id)
    .bind(brand_id)
    .fetch_one(&pool)
    .await
    .expect("fetch upserted row failed");

    assert_eq!(
        status, "succeeded",
        "status should be overwritten to 'succeeded'"
    );
    assert_eq!(
        records_processed, 5,
        "records_processed should be overwritten to 5"
    );
    assert!(
        error_message.is_none(),
        "error_message should be overwritten to NULL"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_collection_run_brands_returns_inserted_entries(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "crb-list", true).await;
    let run = create_collection_run(&pool, "products", "cli")
        .await
        .expect("create_collection_run failed");

    // Insert a brand-level result via the upsert helper
    upsert_collection_run_brand(&pool, run.id, brand_id, "succeeded", Some(3), None)
        .await
        .expect("upsert_collection_run_brand failed");

    let entries = list_collection_run_brands(&pool, run.id)
        .await
        .expect("list_collection_run_brands failed");

    assert_eq!(entries.len(), 1, "should return exactly one entry");
    assert_eq!(entries[0].collection_run_id, run.id);
    assert_eq!(entries[0].brand_id, brand_id);
    assert_eq!(entries[0].status, "succeeded");
    assert_eq!(entries[0].records_processed, 3);
    assert!(entries[0].error_message.is_none());
}

// ---------------------------------------------------------------------------
// Section 8: Store Locations — Dashboard Aggregates
// ---------------------------------------------------------------------------

/// Build a minimal `NewStoreLocation` for tests.
fn make_test_location(
    key_suffix: &str,
    name: &str,
    state: Option<&str>,
    locator_source: Option<&str>,
) -> NewStoreLocation {
    NewStoreLocation {
        location_key: format!("test-loc-{key_suffix}"),
        name: name.to_string(),
        address_line1: None,
        city: None,
        state: state.map(str::to_string),
        zip: None,
        country: Some("US".to_string()),
        latitude: None,
        longitude: None,
        phone: None,
        external_id: None,
        locator_source: locator_source.map(str::to_string),
        raw_data: serde_json::json!({}),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn locations_dashboard_summary_returns_per_brand_counts(pool: sqlx::PgPool) {
    let brand_a = insert_test_brand(&pool, "loc-brand-a", true).await;
    let brand_b = insert_test_brand(&pool, "loc-brand-b", true).await;

    // Brand-a: 3 locations across CA (×2) and TX.
    let locs_a = vec![
        make_test_location("a-ca-1", "Store CA 1", Some("CA"), Some("locally")),
        make_test_location("a-ca-2", "Store CA 2", Some("CA"), Some("locally")),
        make_test_location("a-tx-1", "Store TX 1", Some("TX"), Some("locally")),
    ];
    upsert_store_locations(&pool, brand_a, &locs_a)
        .await
        .expect("upsert brand-a locations failed");

    // Brand-b: 1 location in NY.
    let locs_b = vec![make_test_location(
        "b-ny-1",
        "Store NY 1",
        Some("NY"),
        Some("storemapper"),
    )];
    upsert_store_locations(&pool, brand_b, &locs_b)
        .await
        .expect("upsert brand-b locations failed");

    let rows = list_locations_dashboard_summary(&pool)
        .await
        .expect("list_locations_dashboard_summary failed");

    assert_eq!(rows.len(), 2, "expected 2 brand rows");

    // Results are ordered by active_count DESC, so brand-a (3) comes first.
    let row_a = &rows[0];
    assert_eq!(row_a.brand_slug, "loc-brand-a");
    assert_eq!(row_a.active_count, 3, "brand-a active_count");
    assert_eq!(row_a.states_covered, 2, "brand-a covers CA + TX = 2 states");
    assert_eq!(
        row_a.locator_source.as_deref(),
        Some("locally"),
        "brand-a locator_source"
    );

    let row_b = &rows[1];
    assert_eq!(row_b.brand_slug, "loc-brand-b");
    assert_eq!(row_b.active_count, 1, "brand-b active_count");
    assert_eq!(row_b.states_covered, 1, "brand-b covers NY = 1 state");
    assert_eq!(
        row_b.locator_source.as_deref(),
        Some("storemapper"),
        "brand-b locator_source"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn locations_dashboard_summary_excludes_brands_with_no_active_locations(pool: sqlx::PgPool) {
    // Brand with no locations at all.
    insert_test_brand(&pool, "loc-empty", true).await;

    // Brand with one location that gets deactivated.
    let brand_deactivated = insert_test_brand(&pool, "loc-deactivated", true).await;
    let locs = vec![make_test_location(
        "d-ca-1",
        "Deactivated Store",
        Some("CA"),
        None,
    )];
    upsert_store_locations(&pool, brand_deactivated, &locs)
        .await
        .expect("upsert failed");

    // Deactivate all locations for the brand by passing an empty active_keys slice.
    deactivate_missing_locations(&pool, brand_deactivated, &[])
        .await
        .expect("deactivate failed");

    let rows = list_locations_dashboard_summary(&pool)
        .await
        .expect("list_locations_dashboard_summary failed");

    assert_eq!(
        rows.len(),
        0,
        "HAVING clause should exclude brands with 0 active locations"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn locations_by_state_groups_correctly(pool: sqlx::PgPool) {
    let brand_a = insert_test_brand(&pool, "state-brand-a", true).await;
    let brand_b = insert_test_brand(&pool, "state-brand-b", true).await;

    // brand-a: CA, CA, TX
    let locs_a = vec![
        make_test_location("s-a-ca-1", "Store A-CA-1", Some("CA"), None),
        make_test_location("s-a-ca-2", "Store A-CA-2", Some("CA"), None),
        make_test_location("s-a-tx-1", "Store A-TX-1", Some("TX"), None),
    ];
    upsert_store_locations(&pool, brand_a, &locs_a)
        .await
        .expect("upsert brand-a failed");

    // brand-b: CA, NY
    let locs_b = vec![
        make_test_location("s-b-ca-1", "Store B-CA-1", Some("CA"), None),
        make_test_location("s-b-ny-1", "Store B-NY-1", Some("NY"), None),
    ];
    upsert_store_locations(&pool, brand_b, &locs_b)
        .await
        .expect("upsert brand-b failed");

    let rows = list_locations_by_state(&pool)
        .await
        .expect("list_locations_by_state failed");

    // Ordered by location_count DESC: CA(3), TX(1), NY(1) — TX/NY order may vary.
    let ca_row = rows
        .iter()
        .find(|r| r.state == "CA")
        .expect("CA row missing");
    assert_eq!(ca_row.location_count, 3, "CA location_count");
    assert_eq!(ca_row.brand_count, 2, "CA brand_count");

    let tx_row = rows
        .iter()
        .find(|r| r.state == "TX")
        .expect("TX row missing");
    assert_eq!(tx_row.location_count, 1, "TX location_count");
    assert_eq!(tx_row.brand_count, 1, "TX brand_count");

    let ny_row = rows
        .iter()
        .find(|r| r.state == "NY")
        .expect("NY row missing");
    assert_eq!(ny_row.location_count, 1, "NY location_count");
    assert_eq!(ny_row.brand_count, 1, "NY brand_count");

    // CA must come first (highest count).
    assert_eq!(
        rows[0].state, "CA",
        "CA should be first (location_count DESC)"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn locations_by_state_excludes_null_and_empty_states(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "null-state-brand", true).await;

    // One location with state=None, one with state=Some("").
    let locs = vec![
        make_test_location("ns-null", "Null State Store", None, None),
        make_test_location("ns-empty", "Empty State Store", Some(""), None),
    ];
    upsert_store_locations(&pool, brand_id, &locs)
        .await
        .expect("upsert failed");

    let rows = list_locations_by_state(&pool)
        .await
        .expect("list_locations_by_state failed");

    assert_eq!(
        rows.len(),
        0,
        "rows with NULL or empty state should be excluded by WHERE clause"
    );
}

// ---------------------------------------------------------------------------
// Section 9: Brand Profiles
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn brand_profile_round_trip(pool: sqlx::PgPool) {
    let brand_id: i64 = sqlx::query_scalar(
        "INSERT INTO brands (name, slug, relationship, tier, is_active) \
         VALUES ('TestCo', 'testco', 'competitor', 1, true) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    upsert_brand_profile(
        &pool,
        brand_id,
        Some("Test tagline"),
        Some("Test desc"),
        Some(2021_i16),
        Some("Portland"),
        Some("OR"),
        Some("BigCorp"),
    )
    .await
    .unwrap();

    let profile = scbdb_db::get_brand_profile(&pool, brand_id)
        .await
        .unwrap()
        .expect("profile exists");

    assert_eq!(profile.tagline.as_deref(), Some("Test tagline"));
    assert_eq!(profile.founded_year, Some(2021_i16));
    assert_eq!(profile.hq_state.as_deref(), Some("OR"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_brands_without_profiles_returns_unprofiled(pool: sqlx::PgPool) {
    sqlx::query(
        "INSERT INTO brands (name, slug, relationship, tier, is_active) \
         VALUES ('NoBrand', 'nobrand', 'competitor', 2, true)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let ids = list_brands_without_profiles(&pool).await.unwrap();
    assert!(!ids.is_empty(), "should find brand without profile");
}

// ---------------------------------------------------------------------------
// Section 10: Brand Signals
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn brand_signal_upsert_and_list(pool: sqlx::PgPool) {
    let brand_id: i64 = sqlx::query_scalar(
        "INSERT INTO brands (name, slug, relationship, tier, is_active) \
         VALUES ('SigBrand', 'sigbrand', 'competitor', 1, true) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let new = scbdb_db::NewBrandSignal {
        brand_id,
        signal_type: "article",
        source_platform: Some("web"),
        source_url: Some("https://example.com/article"),
        external_id: Some("art-001"),
        title: Some("Big News"),
        summary: None,
        content: None,
        image_url: None,
        qdrant_point_id: None,
        published_at: None,
    };
    scbdb_db::upsert_brand_signal(&pool, &new).await.unwrap();

    let signals = scbdb_db::list_brand_signals(&pool, brand_id, None, 10, None)
        .await
        .unwrap();
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].title.as_deref(), Some("Big News"));
}

// ---------------------------------------------------------------------------
// Section 11: Brand Funding Events
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn brand_funding_event_round_trip(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "funding-brand", true).await;

    let investors = vec!["Sequoia".to_string(), "a16z".to_string()];
    let event = NewBrandFundingEvent {
        brand_id,
        event_type: "series_a",
        amount_usd: Some(500_000_000), // $5,000,000 in cents
        announced_at: NaiveDate::from_ymd_opt(2025, 6, 15),
        investors: Some(&investors),
        acquirer: None,
        source_url: Some("https://example.com/funding"),
        notes: Some("Series A round"),
    };

    let id = insert_brand_funding_event(&pool, &event)
        .await
        .expect("insert_brand_funding_event failed");
    assert!(id > 0, "returned id should be positive");

    let rows = list_brand_funding_events(&pool, brand_id)
        .await
        .expect("list_brand_funding_events failed");

    assert_eq!(rows.len(), 1, "should return exactly one funding event");
    let row = &rows[0];
    assert_eq!(row.id, id);
    assert_eq!(row.brand_id, brand_id);
    assert_eq!(row.event_type, "series_a");
    assert_eq!(row.amount_usd, Some(500_000_000));
    assert_eq!(row.announced_at, NaiveDate::from_ymd_opt(2025, 6, 15));
    assert_eq!(
        row.investors.as_deref(),
        Some(&["Sequoia".to_string(), "a16z".to_string()][..])
    );
    assert!(row.acquirer.is_none());
    assert_eq!(
        row.source_url.as_deref(),
        Some("https://example.com/funding")
    );
    assert_eq!(row.notes.as_deref(), Some("Series A round"));
}

// ---------------------------------------------------------------------------
// Section 12: Brand Lab Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn brand_lab_test_round_trip(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "lab-brand", true).await;

    let raw = serde_json::json!({"source": "third_party_lab"});
    let test_input = NewBrandLabTest {
        brand_id,
        product_id: None,
        variant_id: None,
        lab_name: Some("SC Labs"),
        test_date: NaiveDate::from_ymd_opt(2025, 4, 10),
        report_url: Some("https://example.com/coa.pdf"),
        thc_mg_actual: Some(Decimal::new(5_250, 3)),
        cbd_mg_actual: Some(Decimal::new(125, 3)),
        total_cannabinoids_mg: Some(Decimal::new(5_375, 3)),
        passed: Some(true),
        raw_data: Some(&raw),
    };

    let id = insert_brand_lab_test(&pool, &test_input)
        .await
        .expect("insert_brand_lab_test failed");
    assert!(id > 0);

    let rows = list_brand_lab_tests(&pool, brand_id)
        .await
        .expect("list_brand_lab_tests failed");

    assert_eq!(rows.len(), 1, "should return exactly one lab test");
    let row = &rows[0];
    assert_eq!(row.id, id);
    assert_eq!(row.brand_id, brand_id);
    assert_eq!(row.lab_name.as_deref(), Some("SC Labs"));
    assert_eq!(row.test_date, NaiveDate::from_ymd_opt(2025, 4, 10));
    assert_eq!(
        row.report_url.as_deref(),
        Some("https://example.com/coa.pdf")
    );
    assert_eq!(row.thc_mg_actual, Some(Decimal::new(5_250, 3)));
    assert_eq!(row.cbd_mg_actual, Some(Decimal::new(125, 3)));
    assert_eq!(row.total_cannabinoids_mg, Some(Decimal::new(5_375, 3)));
    assert_eq!(row.passed, Some(true));
    assert!(row.raw_data.is_some());
}

// ---------------------------------------------------------------------------
// Section 13: Brand Legal Proceedings
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn brand_legal_proceeding_round_trip(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "legal-brand", true).await;

    let proceeding = NewBrandLegalProceeding {
        brand_id,
        proceeding_type: "lawsuit",
        jurisdiction: Some("SC"),
        case_number: Some("2025-CV-001"),
        title: "State v. HempCo",
        summary: Some("Challenge to hemp beverage ban"),
        status: "active",
        filed_at: NaiveDate::from_ymd_opt(2025, 3, 1),
        resolved_at: None,
        source_url: Some("https://example.com/case"),
    };

    let id = insert_brand_legal_proceeding(&pool, &proceeding)
        .await
        .expect("insert_brand_legal_proceeding failed");
    assert!(id > 0);

    let rows = list_brand_legal_proceedings(&pool, brand_id)
        .await
        .expect("list_brand_legal_proceedings failed");

    assert_eq!(rows.len(), 1, "should return exactly one legal proceeding");
    let row = &rows[0];
    assert_eq!(row.id, id);
    assert_eq!(row.brand_id, brand_id);
    assert_eq!(row.proceeding_type, "lawsuit");
    assert_eq!(row.jurisdiction.as_deref(), Some("SC"));
    assert_eq!(row.case_number.as_deref(), Some("2025-CV-001"));
    assert_eq!(row.title, "State v. HempCo");
    assert_eq!(
        row.summary.as_deref(),
        Some("Challenge to hemp beverage ban")
    );
    assert_eq!(row.status, "active");
    assert_eq!(row.filed_at, NaiveDate::from_ymd_opt(2025, 3, 1));
    assert!(row.resolved_at.is_none());
    assert_eq!(row.source_url.as_deref(), Some("https://example.com/case"));
}

// ---------------------------------------------------------------------------
// Section 14: Brand Sponsorships
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn brand_sponsorship_round_trip(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "sponsor-brand", true).await;

    let sponsorship = NewBrandSponsorship {
        brand_id,
        entity_name: "Coastal Music Festival",
        entity_type: "event",
        deal_type: "title_sponsor",
        announced_at: NaiveDate::from_ymd_opt(2025, 5, 1),
        ends_at: NaiveDate::from_ymd_opt(2025, 9, 30),
        source_url: Some("https://example.com/sponsor"),
        notes: Some("Title sponsorship for summer series"),
        is_active: true,
    };

    let id = insert_brand_sponsorship(&pool, &sponsorship)
        .await
        .expect("insert_brand_sponsorship failed");
    assert!(id > 0);

    let rows = list_brand_sponsorships(&pool, brand_id)
        .await
        .expect("list_brand_sponsorships failed");

    assert_eq!(rows.len(), 1, "should return exactly one sponsorship");
    let row = &rows[0];
    assert_eq!(row.id, id);
    assert_eq!(row.brand_id, brand_id);
    assert_eq!(row.entity_name, "Coastal Music Festival");
    assert_eq!(row.entity_type, "event");
    assert_eq!(row.deal_type, "title_sponsor");
    assert_eq!(row.announced_at, NaiveDate::from_ymd_opt(2025, 5, 1));
    assert_eq!(row.ends_at, NaiveDate::from_ymd_opt(2025, 9, 30));
    assert_eq!(
        row.source_url.as_deref(),
        Some("https://example.com/sponsor")
    );
    assert_eq!(
        row.notes.as_deref(),
        Some("Title sponsorship for summer series")
    );
    assert!(row.is_active);
}
