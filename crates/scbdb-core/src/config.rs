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

    let env = parse_environment(&or_default("SCBDB_ENV", "development"))?;

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

    if db_min_connections > db_max_connections {
        return Err(ConfigError::InvalidEnvVar {
            var: "SCBDB_DB_MIN_CONNECTIONS".to_string(),
            reason: format!(
                "must be <= SCBDB_DB_MAX_CONNECTIONS ({db_max_connections}), got {db_min_connections}"
            ),
        });
    }

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
fn parse_environment(s: &str) -> Result<Environment, ConfigError> {
    match s.to_ascii_lowercase().as_str() {
        "development" => Ok(Environment::Development),
        "production" => Ok(Environment::Production),
        "test" => Ok(Environment::Test),
        _ => Err(ConfigError::InvalidEnvVar {
            var: "SCBDB_ENV".to_string(),
            reason: format!(
                "unsupported value '{s}'. Expected one of: development, test, production"
            ),
        }),
    }
}

#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
