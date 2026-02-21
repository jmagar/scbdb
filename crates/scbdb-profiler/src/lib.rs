//! Brand intelligence profiler -- collects and embeds brand signals.

pub mod embedder;
pub mod error;
pub mod intake;
pub mod rss;
pub mod twitter;
pub mod types;
pub mod youtube;

pub use error::ProfilerError;
pub use types::{BrandProfileRunResult, CollectedSignal};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_are_importable() {
        // Verifies the crate compiles and key types exist.
        let _ = std::mem::size_of::<CollectedSignal>();
        let _ = std::mem::size_of::<BrandProfileRunResult>();
        let _ = std::mem::size_of::<ProfilerError>();
    }
}
