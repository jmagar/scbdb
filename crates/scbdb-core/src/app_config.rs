use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Test => write!(f, "test"),
            Environment::Production => write!(f, "production"),
        }
    }
}

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub env: Environment,
    pub bind_addr: SocketAddr,
    pub log_level: String,
    pub brands_path: PathBuf,
    pub api_key_hash_salt: Option<String>,
    pub legiscan_api_key: Option<String>,
    pub db_max_connections: u32,
    pub db_min_connections: u32,
    pub db_acquire_timeout_secs: u64,
    pub scraper_request_timeout_secs: u64,
    pub scraper_user_agent: String,
    pub scraper_max_concurrent_brands: usize,
    pub scraper_inter_request_delay_ms: u64,
    pub scraper_max_retries: u32,
    pub scraper_retry_backoff_base_secs: u64,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("env", &self.env)
            .field("bind_addr", &self.bind_addr)
            .field("log_level", &self.log_level)
            .field("brands_path", &self.brands_path)
            .field("database_url", &"[redacted]")
            .field(
                "api_key_hash_salt",
                &self.api_key_hash_salt.as_ref().map(|_| "[redacted]"),
            )
            .field(
                "legiscan_api_key",
                &self.legiscan_api_key.as_ref().map(|_| "[redacted]"),
            )
            .field("db_max_connections", &self.db_max_connections)
            .field("db_min_connections", &self.db_min_connections)
            .field("db_acquire_timeout_secs", &self.db_acquire_timeout_secs)
            .field(
                "scraper_request_timeout_secs",
                &self.scraper_request_timeout_secs,
            )
            .field("scraper_user_agent", &self.scraper_user_agent)
            .field(
                "scraper_max_concurrent_brands",
                &self.scraper_max_concurrent_brands,
            )
            .field(
                "scraper_inter_request_delay_ms",
                &self.scraper_inter_request_delay_ms,
            )
            .field("scraper_max_retries", &self.scraper_max_retries)
            .field(
                "scraper_retry_backoff_base_secs",
                &self.scraper_retry_backoff_base_secs,
            )
            .finish()
    }
}
