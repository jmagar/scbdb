pub mod client;
pub mod error;
pub mod normalize;
pub mod pagination;
pub mod parse;
pub mod types;

pub use client::ShopifyClient;
pub use error::ScraperError;
pub use normalize::normalize_product;
pub use types::{ShopifyProduct, ShopifyProductsResponse, ShopifyVariant};
