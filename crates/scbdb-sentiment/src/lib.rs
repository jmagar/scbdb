/// Placeholder sentiment scorer — always returns a neutral score.
///
/// This is a stub for Phase 4. Callers should not act on its return value
/// for production decisions until a real model is integrated.
#[must_use]
pub fn score_signal(input: &str) -> f32 {
    if input.trim().is_empty() {
        return 0.0;
    }
    0.5
}
