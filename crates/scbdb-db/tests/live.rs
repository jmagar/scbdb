//! Live integration tests for scbdb-db using `#[sqlx::test]`.
//!
//! Each test gets a fresh, fully-migrated Postgres database spun up by the
//! sqlx test harness. The `migrations` path is relative to the crate root
//! (`crates/scbdb-db/`), so `"../../migrations"` resolves to the workspace
//! migration directory.

use scbdb_core::{NormalizedProduct, NormalizedVariant};
use scbdb_db::{
    complete_collection_run, create_collection_run, fail_collection_run, get_brand_by_slug,
    get_collection_run, insert_price_snapshot_if_changed, list_active_brands, start_collection_run,
    upsert_product, upsert_variant,
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
    .unwrap()
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

    let inserted_first =
        insert_price_snapshot_if_changed(&pool, variant_id, run.id, "12.99", None, "USD", None)
            .await
            .expect("first insert failed");
    assert!(inserted_first, "first snapshot should be inserted");

    let inserted_second =
        insert_price_snapshot_if_changed(&pool, variant_id, run.id, "12.99", None, "USD", None)
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

    let inserted_first =
        insert_price_snapshot_if_changed(&pool, variant_id, run.id, "12.99", None, "USD", None)
            .await
            .unwrap();
    assert!(inserted_first);

    let inserted_second =
        insert_price_snapshot_if_changed(&pool, variant_id, run.id, "14.99", None, "USD", None)
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

// ---------------------------------------------------------------------------
// Section 5: Brands Queries (RED phase — will fail until brands module exists)
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
