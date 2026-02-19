#[must_use]
pub fn score_signal(input: &str) -> f32 {
    if input.trim().is_empty() {
        return 0.0;
    }
    0.5
}
