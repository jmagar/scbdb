//! Domain-specific lexicon scorer for hemp/THC beverage sentiment.

/// Domain-specific word weights.
///
/// Keys are lowercase single words. Values in `(0.0, 1.0]` are positive,
/// in `[-1.0, 0.0)` are negative. The final score is clamped to `[-1.0, 1.0]`.
pub(crate) const LEXICON: &[(&str, f32)] = &[
    // Positive signals
    ("great", 0.4),
    ("good", 0.3),
    ("excellent", 0.5),
    ("positive", 0.4),
    ("approved", 0.5),
    ("legal", 0.4),
    ("legitimate", 0.4),
    ("safe", 0.4),
    ("love", 0.5),
    ("loved", 0.5),
    ("best", 0.5),
    ("recommend", 0.4),
    ("quality", 0.3),
    ("delicious", 0.4),
    ("refreshing", 0.4),
    ("popular", 0.3),
    ("growing", 0.3),
    ("thriving", 0.5),
    ("victory", 0.5),
    ("win", 0.4),
    // Negative signals
    ("ban", -0.6),
    ("banned", -0.6),
    ("illegal", -0.7),
    ("recall", -0.7),
    ("dangerous", -0.6),
    ("harmful", -0.6),
    ("lawsuit", -0.5),
    ("fine", -0.3),
    ("shutdown", -0.6),
    ("bad", -0.4),
    ("terrible", -0.6),
    ("worst", -0.6),
    ("failed", -0.4),
    ("failure", -0.4),
    ("problem", -0.3),
    ("concern", -0.3),
    ("warning", -0.4),
    ("restrict", -0.4),
    ("restricted", -0.4),
    ("prohibition", -0.6),
];

/// Score a text string using the domain lexicon.
///
/// Splits text into lowercase words, sums matching weights, and clamps
/// the result to `[-1.0, 1.0]`. Returns `0.0` for empty or unknown text.
#[must_use]
pub fn lexicon_score(text: &str) -> f32 {
    let mut score = 0.0_f32;
    for word in text.split_whitespace() {
        let w = word
            .trim_matches(|c: char| !c.is_alphabetic())
            .to_lowercase();
        for &(lex_word, weight) in LEXICON {
            if w == lex_word {
                score += weight;
                break;
            }
        }
    }
    score.clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_returns_zero() {
        assert_eq!(lexicon_score(""), 0.0);
    }

    #[test]
    fn whitespace_only_returns_zero() {
        assert_eq!(lexicon_score("   "), 0.0);
    }

    #[test]
    fn unknown_text_returns_zero() {
        assert_eq!(lexicon_score("the quick brown fox"), 0.0);
    }

    #[test]
    fn positive_keyword_returns_positive() {
        let score = lexicon_score("this product is great");
        assert!(score > 0.0, "expected positive score, got {score}");
    }

    #[test]
    fn negative_keyword_returns_negative() {
        let score = lexicon_score("product was banned");
        assert!(score < 0.0, "expected negative score, got {score}");
    }

    #[test]
    fn mixed_text_returns_intermediate() {
        let score = lexicon_score("great product but there was a recall");
        // great (+0.4) + recall (-0.7) = -0.3
        assert!(
            score > -1.0 && score < 1.0,
            "expected intermediate score, got {score}"
        );
    }

    #[test]
    fn score_clamps_to_positive_one() {
        // Stack many positives
        let text = "great excellent best love recommend quality win victory approved";
        let score = lexicon_score(text);
        assert_eq!(score, 1.0, "expected score clamped to 1.0, got {score}");
    }

    #[test]
    fn score_clamps_to_negative_one() {
        // Stack many negatives
        let text = "banned illegal recall dangerous harmful lawsuit shutdown worst prohibition";
        let score = lexicon_score(text);
        assert_eq!(score, -1.0, "expected score clamped to -1.0, got {score}");
    }

    #[test]
    fn punctuation_stripped_from_words() {
        // "great!" should match "great"
        let score = lexicon_score("great!");
        assert!(
            score > 0.0,
            "expected positive score for 'great!', got {score}"
        );
    }
}
