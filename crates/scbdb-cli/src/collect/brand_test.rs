use super::*;

/// Insert a minimal brand row for test purposes and return its generated `id`.
async fn insert_test_brand(pool: &sqlx::PgPool, slug: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO brands (name, slug, relationship, tier, shop_url, is_active) \
         VALUES ($1, $2, 'portfolio', 1, $3, true) RETURNING id",
    )
    .bind(format!("Test Brand {slug}"))
    .bind(slug)
    .bind(format!("https://{slug}.com"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("insert_test_brand failed for slug '{slug}': {e}"))
}

fn make_normalized_product(source_product_id: &str) -> scbdb_core::NormalizedProduct {
    scbdb_core::NormalizedProduct {
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

fn make_normalized_variant(source_variant_id: &str) -> scbdb_core::NormalizedVariant {
    scbdb_core::NormalizedVariant {
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

#[sqlx::test(migrations = "../../migrations")]
async fn persist_normalized_products_deduplicates_snapshots(pool: sqlx::PgPool) {
    let brand_id = insert_test_brand(&pool, "dedup-brand").await;

    let run = scbdb_db::create_collection_run(&pool, "products", "cli")
        .await
        .expect("create_collection_run failed");
    scbdb_db::start_collection_run(&pool, run.id)
        .await
        .expect("start_collection_run failed");

    let product = make_normalized_product("DEDUP-PROD-001");

    // First call: should insert 1 product and 1 snapshot.
    let (products, snapshots) =
        persist_normalized_products(&pool, brand_id, run.id, std::slice::from_ref(&product))
            .await
            .expect("first persist_normalized_products failed");
    assert_eq!(
        (products, snapshots),
        (1, 1),
        "first call should insert 1 product and 1 snapshot"
    );

    // Second call with the same price: should process 1 product but insert 0 new snapshots.
    let (products, snapshots) =
        persist_normalized_products(&pool, brand_id, run.id, std::slice::from_ref(&product))
            .await
            .expect("second persist_normalized_products failed");
    assert_eq!(
        (products, snapshots),
        (1, 0),
        "second call with same price should insert 0 new snapshots (dedup)"
    );
}
