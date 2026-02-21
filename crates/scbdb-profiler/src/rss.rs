//! RSS/Atom feed crawler.

use crate::{error::ProfilerError, types::CollectedSignal};

/// Crawl an RSS/Atom feed and return collected signals.
///
/// # Errors
///
/// Returns [`ProfilerError`] on HTTP, parse, or serialization failures.
#[allow(clippy::unused_async)] // stub -- will await in D3
pub async fn crawl_feed(
    _brand_id: i64,
    _feed_url: &str,
) -> Result<Vec<CollectedSignal>, ProfilerError> {
    todo!("D3: implement RSS/Atom crawler")
}
