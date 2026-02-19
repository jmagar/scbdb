pub mod client;
pub mod error;
pub mod normalize;
pub mod pagination;
pub(crate) mod parse;
pub mod rate_limit;
pub mod types;

pub use client::ShopifyClient;
pub use error::ScraperError;
pub use normalize::normalize_product;
pub use types::{ShopifyProduct, ShopifyProductsResponse, ShopifyVariant};
