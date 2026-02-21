//! Signal intake pipeline: dedup, store, and embed collected signals.

use crate::{error::ProfilerError, types::CollectedSignal};

/// Deduplicate, persist, and embed a batch of collected signals.
/// Returns the number of new signals successfully ingested.
///
/// # Errors
///
/// Returns [`ProfilerError`] on database or embedding failures.
#[allow(clippy::unused_async)] // stub -- will await in D6
pub async fn ingest_signals(_signals: &[CollectedSignal]) -> Result<usize, ProfilerError> {
    todo!("D6: implement signal intake pipeline")
}
