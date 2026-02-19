//! Offline unit tests for scbdb-db pool configuration and row types.
//! These tests do not require a live database connection.

use scbdb_db::{CollectionRunRow, PoolConfig, ProductRow};

#[test]
fn pool_config_from_env_uses_defaults_when_vars_unset() {
    // When SCBDB_DB_* env vars are absent, from_env() must return the same
    // values as default(). The SCBDB_DB_* vars are never set by CI or the
    // test harness, so these assertions are unconditional.
    let from_env = PoolConfig::from_env();
    let default = PoolConfig::default();
    assert_eq!(from_env.max_connections, default.max_connections);
    assert_eq!(from_env.min_connections, default.min_connections);
    assert_eq!(from_env.acquire_timeout_secs, default.acquire_timeout_secs);
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
