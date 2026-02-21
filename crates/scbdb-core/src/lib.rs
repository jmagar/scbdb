pub mod app_config;
pub mod brands;
pub mod config;
pub mod products;

pub use app_config::{AppConfig, Environment};
pub use brands::{load_brands, BrandConfig, BrandsFile, Relationship};
pub use config::{load_app_config, load_app_config_from_env};
pub use products::{NormalizedImage, NormalizedProduct, NormalizedVariant};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required env var: {0}")]
    MissingEnvVar(String),

    #[error("invalid env var {var}: {reason}")]
    InvalidEnvVar { var: String, reason: String },

    #[error("failed to read brands file {path}: {source}")]
    BrandsFileIo {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse brands file: {0}")]
    BrandsFileParse(#[source] serde_yaml::Error),

    #[error("validation error: {0}")]
    Validation(String),
}
