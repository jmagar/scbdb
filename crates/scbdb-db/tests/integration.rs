//! Offline unit tests for scbdb-db pool configuration and row types.
//! These tests do not require a live database connection.

use scbdb_core::{AppConfig, Environment};
use scbdb_db::{CollectionRunRow, PoolConfig, ProductRow};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

#[test]
fn pool_config_from_app_config_uses_core_values() {
    let app_config = AppConfig {
        database_url: "postgres://example".to_string(),
        env: Environment::Test,
        bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000),
        log_level: "info".to_string(),
        brands_path: PathBuf::from("./config/brands.yaml"),
        api_key_hash_salt: "salt".to_string(),
        legiscan_api_key: None,
        db_max_connections: 42,
        db_min_connections: 7,
        db_acquire_timeout_secs: 9,
        scraper_request_timeout_secs: 30,
        scraper_user_agent: "ua".to_string(),
        scraper_max_concurrent_brands: 1,
        scraper_inter_request_delay_ms: 250,
        scraper_max_retries: 3,
        scraper_retry_backoff_base_secs: 5,
    };

    let pool_config = PoolConfig::from_app_config(&app_config);
    assert_eq!(pool_config.max_connections, 42);
    assert_eq!(pool_config.min_connections, 7);
    assert_eq!(pool_config.acquire_timeout_secs, 9);
}

/// Compile-time smoke test: confirm that [`CollectionRunRow`] has all expected
/// fields with the correct types. No database required.
#[test]
fn collection_run_row_has_expected_fields() {
    use chrono::Utc;
    use uuid::Uuid;

    let row = CollectionRunRow {
        id: 1_i64,
        public_id: Uuid::new_v4(),
        run_type: "products".to_string(),
        trigger_source: "cli".to_string(),
        status: "queued".to_string(),
        started_at: None,
        completed_at: None,
        records_processed: 0_i32,
        error_message: None,
        created_at: Utc::now(),
    };

    assert_eq!(row.id, 1);
    assert_eq!(row.run_type, "products");
    assert_eq!(row.trigger_source, "cli");
    assert_eq!(row.status, "queued");
    assert!(row.started_at.is_none());
    assert!(row.completed_at.is_none());
    assert_eq!(row.records_processed, 0);
    assert!(row.error_message.is_none());
}

/// Compile-time smoke test: confirm that [`ProductRow`] has all expected
/// fields with the correct types. No database required.
#[test]
fn product_row_has_expected_fields() {
    use chrono::Utc;

    let row = ProductRow {
        id: 42_i64,
        brand_id: 7_i64,
        source_platform: "shopify".to_string(),
        source_product_id: "123456789".to_string(),
        name: "Hi Boy Blood Orange".to_string(),
        status: Some("active".to_string()),
        handle: None,
        source_url: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    assert_eq!(row.id, 42);
    assert_eq!(row.brand_id, 7);
    assert_eq!(row.source_platform, "shopify");
    assert_eq!(row.source_product_id, "123456789");
    assert_eq!(row.name, "Hi Boy Blood Orange");
    assert_eq!(row.status.as_deref(), Some("active"));
    assert!(row.handle.is_none());
}
