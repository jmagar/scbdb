//! Twitter/X profile and tweet signal collector.

use crate::{error::ProfilerError, types::CollectedSignal};

/// Collect signals from a Twitter/X profile's recent activity.
///
/// # Errors
///
/// Returns [`ProfilerError`] on HTTP or serialization failures.
#[allow(clippy::unused_async)] // stub -- will await in D5
pub async fn collect_profile_signals(
    _brand_id: i64,
    _handle: &str,
) -> Result<Vec<CollectedSignal>, ProfilerError> {
    todo!("D5: implement Twitter/X signal collection")
}
