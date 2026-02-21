//! TEI embeddings client and Qdrant point-ID derivation.

use crate::error::ProfilerError;

/// Embeds a signal text and stores the vector in Qdrant.
/// Returns the Qdrant point ID.
///
/// # Errors
///
/// Returns [`ProfilerError`] on HTTP or serialization failures.
#[allow(clippy::unused_async)] // stub -- will await in D2
pub async fn embed_signal(_text: &str, _external_id: &str) -> Result<String, ProfilerError> {
    todo!("D2: implement TEI + Qdrant embedding")
}
