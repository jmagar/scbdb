//! `YouTube` channel and video signal collector.

use crate::{error::ProfilerError, types::CollectedSignal};

/// Collect signals from a `YouTube` channel's recent uploads.
///
/// # Errors
///
/// Returns [`ProfilerError`] on HTTP or serialization failures.
#[allow(clippy::unused_async)] // stub -- will await in D4
pub async fn collect_channel_signals(
    _brand_id: i64,
    _channel_id: &str,
) -> Result<Vec<CollectedSignal>, ProfilerError> {
    todo!("D4: implement YouTube channel signal collection")
}
