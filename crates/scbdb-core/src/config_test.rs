use std::collections::HashMap;
use std::env::VarError;

use super::*;

fn lookup_from_map<'a>(
    map: &'a HashMap<&'a str, &'a str>,
) -> impl Fn(&str) -> Result<String, VarError> + 'a {
    move |key| {
        map.get(key)
            .map(|v| (*v).to_string())
            .ok_or(VarError::NotPresent)
    }
}

/// Returns a map with all required env vars populated with valid defaults.
fn full_env<'a>() -> HashMap<&'a str, &'a str> {
    let mut m = HashMap::new();
    m.insert("DATABASE_URL", "postgres://user:pass@localhost/testdb");
    m.insert("SCBDB_API_KEY_HASH_SALT", "test-salt");
    m
}

#[test]
fn parse_environment_development() {
    assert_eq!(
        parse_environment("development").unwrap(),
        Environment::Development
    );
}

#[test]
fn parse_environment_test() {
    assert_eq!(parse_environment("test").unwrap(), Environment::Test);
}

#[test]
fn parse_environment_production() {
    assert_eq!(
        parse_environment("production").unwrap(),
        Environment::Production
    );
}

#[test]
fn parse_environment_unknown_fails() {
    let err = parse_environment("unknown").unwrap_err();
    assert!(matches!(err, ConfigError::InvalidEnvVar { ref var, .. } if var == "SCBDB_ENV"));
}

#[test]
fn build_app_config_fails_without_database_url() {
    let map: HashMap<&str, &str> = HashMap::new();
    let result = build_app_config(lookup_from_map(&map));
    assert!(
        matches!(result, Err(ConfigError::MissingEnvVar(ref v)) if v == "DATABASE_URL"),
        "expected MissingEnvVar(DATABASE_URL), got: {result:?}"
    );
}

#[test]
fn build_app_config_fails_with_invalid_bind_addr() {
    let mut map = full_env();
    map.insert("SCBDB_BIND_ADDR", "not-a-socket-addr");
    let result = build_app_config(lookup_from_map(&map));
    assert!(
        matches!(result, Err(ConfigError::InvalidEnvVar { ref var, .. }) if var == "SCBDB_BIND_ADDR"),
        "expected InvalidEnvVar(SCBDB_BIND_ADDR), got: {result:?}"
    );
}

#[test]
fn build_app_config_succeeds_with_all_required_vars() {
    let map = full_env();
    let result = build_app_config(lookup_from_map(&map));
    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    let cfg = result.unwrap();
    assert_eq!(cfg.env, Environment::Development);
    assert_eq!(cfg.database_url, "postgres://user:pass@localhost/testdb");
    assert_eq!(cfg.api_key_hash_salt, Some("test-salt".to_string()));
    assert_eq!(cfg.bind_addr.to_string(), "0.0.0.0:3000");
    assert_eq!(cfg.log_level, "info");
    assert_eq!(cfg.db_max_connections, 10);
    assert_eq!(cfg.db_min_connections, 1);
    assert_eq!(cfg.db_acquire_timeout_secs, 10);
    assert!(cfg.legiscan_api_key.is_none());
    assert_eq!(cfg.scraper_request_timeout_secs, 30);
    assert_eq!(cfg.scraper_user_agent, "scbdb/0.1 (product-intelligence)");
    assert_eq!(cfg.scraper_max_concurrent_brands, 1);
    assert_eq!(cfg.scraper_inter_request_delay_ms, 250);
    assert_eq!(cfg.scraper_max_retries, 3);
    assert_eq!(cfg.scraper_retry_backoff_base_secs, 5);
}

#[test]
fn parse_environment_scraper_request_timeout_secs_default() {
    let map = full_env();
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_request_timeout_secs, 30);
}

#[test]
fn parse_environment_scraper_request_timeout_secs_override() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_REQUEST_TIMEOUT_SECS", "60");
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_request_timeout_secs, 60);
}

#[test]
fn parse_environment_scraper_request_timeout_secs_invalid() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_REQUEST_TIMEOUT_SECS", "not-a-number");
    let result = build_app_config(lookup_from_map(&map));
    assert!(
        matches!(result, Err(ConfigError::InvalidEnvVar { ref var, .. }) if var == "SCBDB_SCRAPER_REQUEST_TIMEOUT_SECS"),
        "expected InvalidEnvVar(SCBDB_SCRAPER_REQUEST_TIMEOUT_SECS), got: {result:?}"
    );
}

#[test]
fn parse_environment_scraper_user_agent_default() {
    let map = full_env();
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_user_agent, "scbdb/0.1 (product-intelligence)");
}

#[test]
fn parse_environment_scraper_user_agent_override() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_USER_AGENT", "custom-agent/2.0");
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_user_agent, "custom-agent/2.0");
}

#[test]
fn parse_environment_scraper_max_concurrent_brands_default() {
    let map = full_env();
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_max_concurrent_brands, 1);
}

#[test]
fn parse_environment_scraper_max_concurrent_brands_override() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_MAX_CONCURRENT_BRANDS", "4");
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_max_concurrent_brands, 4);
}

#[test]
fn parse_environment_scraper_max_concurrent_brands_invalid() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_MAX_CONCURRENT_BRANDS", "not-a-number");
    let result = build_app_config(lookup_from_map(&map));
    assert!(
        matches!(result, Err(ConfigError::InvalidEnvVar { ref var, .. }) if var == "SCBDB_SCRAPER_MAX_CONCURRENT_BRANDS"),
        "expected InvalidEnvVar(SCBDB_SCRAPER_MAX_CONCURRENT_BRANDS), got: {result:?}"
    );
}

#[test]
fn parse_environment_scraper_inter_request_delay_ms_default() {
    let map = full_env();
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_inter_request_delay_ms, 250);
}

#[test]
fn parse_environment_scraper_inter_request_delay_ms_override() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_INTER_REQUEST_DELAY_MS", "500");
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_inter_request_delay_ms, 500);
}

#[test]
fn parse_environment_scraper_inter_request_delay_ms_invalid() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_INTER_REQUEST_DELAY_MS", "not-a-number");
    let result = build_app_config(lookup_from_map(&map));
    assert!(
        matches!(result, Err(ConfigError::InvalidEnvVar { ref var, .. }) if var == "SCBDB_SCRAPER_INTER_REQUEST_DELAY_MS"),
        "expected InvalidEnvVar(SCBDB_SCRAPER_INTER_REQUEST_DELAY_MS), got: {result:?}"
    );
}

#[test]
fn parse_environment_scraper_max_retries_default() {
    let map = full_env();
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_max_retries, 3);
}

#[test]
fn parse_environment_scraper_max_retries_override() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_MAX_RETRIES", "5");
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_max_retries, 5);
}

#[test]
fn parse_environment_scraper_max_retries_invalid() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_MAX_RETRIES", "not-a-number");
    let result = build_app_config(lookup_from_map(&map));
    assert!(
        matches!(result, Err(ConfigError::InvalidEnvVar { ref var, .. }) if var == "SCBDB_SCRAPER_MAX_RETRIES"),
        "expected InvalidEnvVar(SCBDB_SCRAPER_MAX_RETRIES), got: {result:?}"
    );
}

#[test]
fn parse_environment_scraper_retry_backoff_base_secs_default() {
    let map = full_env();
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_retry_backoff_base_secs, 5);
}

#[test]
fn parse_environment_scraper_retry_backoff_base_secs_override() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_RETRY_BACKOFF_BASE_SECS", "10");
    let cfg = build_app_config(lookup_from_map(&map)).unwrap();
    assert_eq!(cfg.scraper_retry_backoff_base_secs, 10);
}

#[test]
fn parse_environment_scraper_retry_backoff_base_secs_invalid() {
    let mut map = full_env();
    map.insert("SCBDB_SCRAPER_RETRY_BACKOFF_BASE_SECS", "not-a-number");
    let result = build_app_config(lookup_from_map(&map));
    assert!(
        matches!(result, Err(ConfigError::InvalidEnvVar { ref var, .. }) if var == "SCBDB_SCRAPER_RETRY_BACKOFF_BASE_SECS"),
        "expected InvalidEnvVar(SCBDB_SCRAPER_RETRY_BACKOFF_BASE_SECS), got: {result:?}"
    );
}

#[test]
fn build_app_config_fails_on_invalid_scbdb_env() {
    let mut map = full_env();
    map.insert("SCBDB_ENV", "producton");
    let result = build_app_config(lookup_from_map(&map));
    assert!(
        matches!(result, Err(ConfigError::InvalidEnvVar { ref var, .. }) if var == "SCBDB_ENV"),
        "expected InvalidEnvVar(SCBDB_ENV), got: {result:?}"
    );
}

#[test]
fn build_app_config_fails_when_db_min_exceeds_db_max() {
    let mut map = full_env();
    map.insert("SCBDB_DB_MIN_CONNECTIONS", "11");
    map.insert("SCBDB_DB_MAX_CONNECTIONS", "10");
    let result = build_app_config(lookup_from_map(&map));
    assert!(
        matches!(result, Err(ConfigError::InvalidEnvVar { ref var, .. }) if var == "SCBDB_DB_MIN_CONNECTIONS"),
        "expected InvalidEnvVar(SCBDB_DB_MIN_CONNECTIONS), got: {result:?}"
    );
}
