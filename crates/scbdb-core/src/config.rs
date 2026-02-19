use crate::app_config::{AppConfig, Environment};
use crate::ConfigError;

/// Load application configuration from environment variables.
///
/// Calls `dotenvy::dotenv().ok()` to load `.env` files before reading env vars.
///
/// # Errors
///
/// Returns `ConfigError` if required env vars are missing or values are invalid.
pub fn load_app_config() -> Result<AppConfig, ConfigError> {
    dotenvy::dotenv().ok();
    load_app_config_from_env()
}

/// Load application configuration from environment variables already in the process.
///
/// Unlike [`load_app_config`], this does NOT load `.env` files — useful for testing
/// or when the caller manages env setup.
///
/// # Errors
///
/// Returns `ConfigError` if required env vars are missing or values are invalid.
pub fn load_app_config_from_env() -> Result<AppConfig, ConfigError> {
    build_app_config(|key| std::env::var(key))
}

/// Build application configuration using the provided env-var lookup function.
///
/// This is the core parsing/validation logic, decoupled from the actual environment
/// so it can be tested with a pure `HashMap` lookup — no `set_var`/`remove_var` needed.
fn build_app_config<F>(lookup: F) -> Result<AppConfig, ConfigError>
where
    F: Fn(&str) -> Result<String, std::env::VarError>,
{
    use std::net::SocketAddr;
    use std::path::PathBuf;

    let require = |var: &str| -> Result<String, ConfigError> {
        lookup(var).map_err(|_| ConfigError::MissingEnvVar(var.to_string()))
    };

    let or_default = |var: &str, default: &str| -> String {
        lookup(var).unwrap_or_else(|_| default.to_string())
    };

    let parse = |var: &str, default: &str| -> Result<SocketAddr, ConfigError> {
        let raw = or_default(var, default);
        raw.parse::<SocketAddr>()
            .map_err(|e| ConfigError::InvalidEnvVar {
                var: var.to_string(),
                reason: e.to_string(),
            })
    };

    let parse_u32 = |var: &str, default: &str| -> Result<u32, ConfigError> {
        let raw = or_default(var, default);
        raw.parse::<u32>().map_err(|e| ConfigError::InvalidEnvVar {
            var: var.to_string(),
            reason: e.to_string(),
        })
    };

    let parse_u64 = |var: &str, default: &str| -> Result<u64, ConfigError> {
        let raw = or_default(var, default);
        raw.parse::<u64>().map_err(|e| ConfigError::InvalidEnvVar {
            var: var.to_string(),
            reason: e.to_string(),
        })
    };

    let parse_usize = |var: &str, default: &str| -> Result<usize, ConfigError> {
        let raw = or_default(var, default);
        raw.parse::<usize>()
            .map_err(|e| ConfigError::InvalidEnvVar {
                var: var.to_string(),
                reason: e.to_string(),
            })
    };

    let database_url = require("DATABASE_URL")?;
    let api_key_hash_salt = require("SCBDB_API_KEY_HASH_SALT")?;

    let env = parse_environment(&or_default("SCBDB_ENV", "development"));

    let bind_addr = parse("SCBDB_BIND_ADDR", "0.0.0.0:3000")?;
    let log_level = or_default("SCBDB_LOG_LEVEL", "info");
    let brands_path = PathBuf::from(or_default("SCBDB_BRANDS_PATH", "./config/brands.yaml"));
    let legiscan_api_key = lookup("LEGISCAN_API_KEY").ok();

    let db_max_connections = parse_u32("SCBDB_DB_MAX_CONNECTIONS", "10")?;
    let db_min_connections = parse_u32("SCBDB_DB_MIN_CONNECTIONS", "1")?;
    let db_acquire_timeout_secs = parse_u64("SCBDB_DB_ACQUIRE_TIMEOUT_SECS", "10")?;

    let scraper_request_timeout_secs = parse_u64("SCBDB_SCRAPER_REQUEST_TIMEOUT_SECS", "30")?;
    let scraper_user_agent = or_default(
        "SCBDB_SCRAPER_USER_AGENT",
        "scbdb/0.1 (product-intelligence)",
    );
    let scraper_max_concurrent_brands = parse_usize("SCBDB_SCRAPER_MAX_CONCURRENT_BRANDS", "1")?;
    let scraper_inter_request_delay_ms = parse_u64("SCBDB_SCRAPER_INTER_REQUEST_DELAY_MS", "250")?;
    let scraper_max_retries = parse_u32("SCBDB_SCRAPER_MAX_RETRIES", "3")?;
    let scraper_retry_backoff_base_secs = parse_u64("SCBDB_SCRAPER_RETRY_BACKOFF_BASE_SECS", "5")?;

    Ok(AppConfig {
        database_url,
        env,
        bind_addr,
        log_level,
        brands_path,
        api_key_hash_salt,
        legiscan_api_key,
        db_max_connections,
        db_min_connections,
        db_acquire_timeout_secs,
        scraper_request_timeout_secs,
        scraper_user_agent,
        scraper_max_concurrent_brands,
        scraper_inter_request_delay_ms,
        scraper_max_retries,
        scraper_retry_backoff_base_secs,
    })
}

/// Parse a string into an `Environment` variant.
///
/// Unrecognized values default to `Environment::Development`.
fn parse_environment(s: &str) -> Environment {
    match s {
        "production" => Environment::Production,
        "test" => Environment::Test,
        _ => Environment::Development,
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(parse_environment("development"), Environment::Development);
    }

    #[test]
    fn parse_environment_test() {
        assert_eq!(parse_environment("test"), Environment::Test);
    }

    #[test]
    fn parse_environment_production() {
        assert_eq!(parse_environment("production"), Environment::Production);
    }

    #[test]
    fn parse_environment_unknown_defaults_to_development() {
        assert_eq!(parse_environment("unknown"), Environment::Development);
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
    fn build_app_config_fails_without_api_key_hash_salt() {
        let mut map: HashMap<&str, &str> = HashMap::new();
        map.insert("DATABASE_URL", "postgres://user:pass@localhost/testdb");
        let result = build_app_config(lookup_from_map(&map));
        assert!(
            matches!(result, Err(ConfigError::MissingEnvVar(ref v)) if v == "SCBDB_API_KEY_HASH_SALT"),
            "expected MissingEnvVar(SCBDB_API_KEY_HASH_SALT), got: {result:?}"
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
}
