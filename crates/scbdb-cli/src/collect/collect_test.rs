use super::*;

/// Insert a minimal brand row for test purposes.
///
/// When `shop_url` is `None` the brand is inserted without a shop URL.
async fn insert_test_brand(pool: &sqlx::PgPool, slug: &str, shop_url: Option<&str>) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO brands (name, slug, relationship, tier, shop_url, is_active) \
         VALUES ($1, $2, 'portfolio', 1, $3, true) RETURNING id",
    )
    .bind(format!("Test Brand {slug}"))
    .bind(slug)
    .bind(shop_url)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("insert_test_brand failed for slug '{slug}': {e}"))
}

#[sqlx::test(migrations = "../../migrations")]
async fn load_brands_unknown_slug_returns_error(pool: sqlx::PgPool) {
    // Insert a brand with a different slug so the table isn't empty.
    insert_test_brand(&pool, "existing-brand", Some("https://existing.com")).await;

    let result = load_brands_for_collect(&pool, Some("nonexistent")).await;

    let err = result.expect_err("expected Err for unknown slug");
    let msg = format!("{err}");
    assert!(
        msg.contains("not found"),
        "error should mention 'not found', got: {msg}"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn load_brands_known_slug_null_shop_url_returns_error(pool: sqlx::PgPool) {
    insert_test_brand(&pool, "no-shop", None).await;

    let result = load_brands_for_collect(&pool, Some("no-shop")).await;

    let err = result.expect_err("expected Err for brand with null shop_url");
    let msg = format!("{err}");
    assert!(
        msg.contains("no shop_url"),
        "error should mention 'no shop_url', got: {msg}"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn load_brands_all_filters_out_null_shop_url(pool: sqlx::PgPool) {
    insert_test_brand(&pool, "has-shop", Some("https://has-shop.com")).await;
    insert_test_brand(&pool, "no-shop-all", None).await;

    let brands = load_brands_for_collect(&pool, None)
        .await
        .expect("load_brands_for_collect should succeed");

    assert_eq!(
        brands.len(),
        1,
        "only the brand with shop_url should be returned"
    );
    assert_eq!(brands[0].slug, "has-shop");
}

#[sqlx::test(migrations = "../../migrations")]
async fn run_collect_products_dry_run_writes_zero_db_rows(pool: sqlx::PgPool) {
    insert_test_brand(&pool, "dry-run-brand", Some("https://dry-run.com")).await;

    let config = scbdb_core::AppConfig {
        database_url: String::new(),
        env: scbdb_core::Environment::Test,
        bind_addr: "0.0.0.0:3000".parse().unwrap(),
        log_level: "info".to_string(),
        brands_path: std::path::PathBuf::from("config/brands.yaml"),
        api_key_hash_salt: None,
        legiscan_api_key: None,
        db_max_connections: 10,
        db_min_connections: 1,
        db_acquire_timeout_secs: 10,
        scraper_request_timeout_secs: 30,
        legiscan_request_timeout_secs: 30,
        scraper_user_agent: "scbdb/0.1 (test)".to_string(),
        scraper_max_concurrent_brands: 1,
        scraper_inter_request_delay_ms: 0,
        scraper_max_retries: 3,
        scraper_retry_backoff_base_secs: 5,
    };

    let result = run_collect_products(&pool, &config, None, true).await;
    assert!(
        result.is_ok(),
        "dry-run should return Ok(()), got: {result:?}"
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_runs")
        .fetch_one(&pool)
        .await
        .expect("count query failed");

    assert_eq!(count, 0, "dry-run must not create any collection_runs rows");
}
