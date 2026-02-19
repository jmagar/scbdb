//! Low-level string parsing helpers for extracting dosage and size from variant titles.
//!
//! These functions use manual byte scanning rather than `regex` to stay
//! dependency-light. See [`crate::normalize`] for how they compose into full
//! product normalization.

/// Attempts to parse a THC dosage value in milligrams from a variant title.
///
/// Matching rules (case-insensitive):
/// 1. `"5mg thc"` / `"5 mg thc"` — explicit THC label → definitive match.
/// 2. `"5mg"` with no `"cbd"` nearby — treat as THC (beverage convention).
/// 3. `"thc 5mg"` — label precedes value.
///
/// Returns `None` when no parseable pattern is found.
#[must_use]
pub(crate) fn parse_thc_mg(title: &str) -> Option<f64> {
    let lower = title.to_lowercase();
    parse_mg_with_label(&lower, "thc").or_else(|| {
        // Bare "Nmg" with no CBD label: attribute to THC.
        if lower.contains("cbd") {
            None
        } else {
            parse_bare_mg(&lower)
        }
    })
}

/// Attempts to parse a CBD content value in milligrams from a variant title.
///
/// Matching rules (case-insensitive):
/// - `"2mg cbd"` / `"2 mg cbd"` / `"cbd 2mg"` — explicit CBD label required.
///
/// Returns `None` when no parseable pattern is found.
#[must_use]
pub(crate) fn parse_cbd_mg(title: &str) -> Option<f64> {
    let lower = title.to_lowercase();
    parse_mg_with_label(&lower, "cbd")
}

/// Attempts to parse a volume or pack size from a variant title.
///
/// Recognizes:
/// - Fluid ounces: `"12oz"`, `"12 oz"`, `"8.5oz"`
/// - Millilitres: `"355ml"`, `"355 ml"`
///
/// Returns `Some((value, unit))` or `None` if no size is found.
#[must_use]
pub(crate) fn parse_size(title: &str) -> Option<(f64, String)> {
    let lower = title.to_lowercase();
    parse_size_unit(&lower, "oz").or_else(|| parse_size_unit(&lower, "ml"))
}

// ---------------------------------------------------------------------------
// Internal parsing helpers
// ---------------------------------------------------------------------------

/// Parses a dosage value where `label` (e.g., `"thc"` or `"cbd"`) appears
/// either immediately before or after the `mg` number.
///
/// Handles: `"5mg thc"`, `"5 mg thc"`, `"thc 5mg"`, `"thc 5 mg"`.
/// Input must be pre-lowercased.
///
/// Strategy: split the window around the label.
/// - **Before label** (e.g., `"6mg CBD"`): collect all mg values before the
///   label and return the *last* one (closest to the label).
/// - **After label** (e.g., `"CBD 6mg"`): return the first mg value after
///   the label.
///
/// This correctly handles combined titles like `"3mg THC, 6mg CBD"` where
/// the before-label scan finds both 3mg and 6mg but returns 6mg (last/closest).
fn parse_mg_with_label(lower: &str, label: &str) -> Option<f64> {
    let window = 20usize;
    let mut search_from = 0usize;

    loop {
        let rel_pos = lower[search_from..].find(label)?;
        let label_pos = search_from + rel_pos;

        let before_start = label_pos.saturating_sub(window);
        let after_end = (label_pos + label.len() + window).min(lower.len());

        // Last mg before the label (closest to it).
        let before_slice = &lower[before_start..label_pos];
        if let Some(value) = all_mg_values(before_slice).into_iter().last() {
            return Some(value);
        }

        // First mg after the label.
        let after_slice = &lower[label_pos + label.len()..after_end];
        if let Some(value) = extract_mg_value(after_slice) {
            return Some(value);
        }

        search_from = label_pos + label.len();
    }
}

/// Parses a bare `"Nmg"` or `"N mg"` with no label required.
/// Input must be pre-lowercased.
fn parse_bare_mg(lower: &str) -> Option<f64> {
    extract_mg_value(lower)
}

/// Returns all `Nmg` values found in `s` in left-to-right order.
/// Input must be pre-lowercased.
fn all_mg_values(s: &str) -> Vec<f64> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut values = Vec::new();
    let mut i = 0;

    while i < len {
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit())
        {
            let num_start = i;
            let mut has_dot = false;
            while i < len && (bytes[i].is_ascii_digit() || (bytes[i] == b'.' && !has_dot)) {
                if bytes[i] == b'.' {
                    has_dot = true;
                }
                i += 1;
            }
            let num_str = &s[num_start..i];
            let after_num = i;
            let mut scan = i;
            while scan < len && bytes[scan] == b' ' {
                scan += 1;
            }
            if s[scan..].starts_with("mg") {
                if let Ok(v) = num_str.parse::<f64>() {
                    values.push(v);
                }
                i = scan.saturating_add(2);
                continue;
            }
            i = after_num;
        } else {
            i += 1;
        }
    }
    values
}

/// Scans `s` for the first occurrence of a number (integer or decimal)
/// optionally followed by whitespace and then `"mg"`. Returns the parsed
/// `f64` value or `None`.
fn extract_mg_value(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;

    while i < len {
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit())
        {
            let num_start = i;

            let mut has_dot = false;
            while i < len && (bytes[i].is_ascii_digit() || (bytes[i] == b'.' && !has_dot)) {
                if bytes[i] == b'.' {
                    has_dot = true;
                }
                i += 1;
            }
            let num_str = &s[num_start..i];

            let after_num = i;
            while i < len && bytes[i] == b' ' {
                i += 1;
            }

            if s[i..].starts_with("mg") {
                if let Ok(v) = num_str.parse::<f64>() {
                    return Some(v);
                }
            }

            i = after_num;
        } else {
            i += 1;
        }
    }
    None
}

/// Parses a size value followed by `unit` (e.g., `"oz"` or `"ml"`).
/// Input must be pre-lowercased.
fn parse_size_unit(lower: &str, unit: &str) -> Option<(f64, String)> {
    let mut i = 0usize;
    let bytes = lower.as_bytes();
    let len = bytes.len();

    while i < len {
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit())
        {
            let num_start = i;
            let mut has_dot = false;
            while i < len && (bytes[i].is_ascii_digit() || (bytes[i] == b'.' && !has_dot)) {
                if bytes[i] == b'.' {
                    has_dot = true;
                }
                i += 1;
            }
            let num_str = &lower[num_start..i];

            while i < len && bytes[i] == b' ' {
                i += 1;
            }

            if lower[i..].starts_with(unit) {
                if let Ok(v) = num_str.parse::<f64>() {
                    return Some((v, unit.to_owned()));
                }
            }
        } else {
            i += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // parse_thc_mg
    // -----------------------------------------------------------------------

    #[test]
    fn thc_mg_explicit_label_no_space() {
        assert_eq!(parse_thc_mg("5mg THC"), Some(5.0));
    }

    #[test]
    fn thc_mg_explicit_label_with_space() {
        assert_eq!(parse_thc_mg("5 mg THC"), Some(5.0));
    }

    #[test]
    fn thc_mg_label_before_value() {
        assert_eq!(parse_thc_mg("THC 10mg"), Some(10.0));
    }

    #[test]
    fn thc_mg_label_before_value_with_space() {
        assert_eq!(parse_thc_mg("THC 10 mg"), Some(10.0));
    }

    #[test]
    fn thc_mg_bare_no_label() {
        assert_eq!(parse_thc_mg("12oz / 5mg"), Some(5.0));
    }

    #[test]
    fn thc_mg_bare_ignored_when_cbd_present() {
        assert_eq!(parse_thc_mg("5mg THC, 2mg CBD"), Some(5.0));
    }

    #[test]
    fn thc_mg_case_insensitive() {
        assert_eq!(parse_thc_mg("10MG thc"), Some(10.0));
    }

    #[test]
    fn thc_mg_decimal_value() {
        assert_eq!(parse_thc_mg("2.5mg THC"), Some(2.5));
    }

    #[test]
    fn thc_mg_not_present_returns_none() {
        assert!(parse_thc_mg("Hi Boy").is_none());
    }

    #[test]
    fn thc_mg_default_title_returns_none() {
        assert!(parse_thc_mg("Default Title").is_none());
    }

    #[test]
    fn thc_mg_complex_title() {
        assert_eq!(parse_thc_mg("12oz / 5mg THC"), Some(5.0));
    }

    // -----------------------------------------------------------------------
    // parse_cbd_mg
    // -----------------------------------------------------------------------

    #[test]
    fn cbd_mg_explicit_label_no_space() {
        assert_eq!(parse_cbd_mg("2mg CBD"), Some(2.0));
    }

    #[test]
    fn cbd_mg_explicit_label_with_space() {
        assert_eq!(parse_cbd_mg("2 mg CBD"), Some(2.0));
    }

    #[test]
    fn cbd_mg_label_before_value() {
        assert_eq!(parse_cbd_mg("CBD 6mg"), Some(6.0));
    }

    #[test]
    fn cbd_mg_decimal_value() {
        assert_eq!(parse_cbd_mg("1.5mg CBD"), Some(1.5));
    }

    #[test]
    fn cbd_mg_case_insensitive() {
        assert_eq!(parse_cbd_mg("2MG cbd"), Some(2.0));
    }

    #[test]
    fn cbd_mg_not_present_returns_none() {
        assert!(parse_cbd_mg("12oz / 5mg THC").is_none());
    }

    #[test]
    fn cbd_mg_bare_mg_without_label_returns_none() {
        assert!(parse_cbd_mg("5mg").is_none());
    }

    #[test]
    fn cbd_mg_combined_title() {
        // "3mg THC, 6mg CBD" — must return the CBD value (6mg), not the THC value (3mg).
        assert_eq!(parse_cbd_mg("3mg THC, 6mg CBD"), Some(6.0));
    }

    // -----------------------------------------------------------------------
    // parse_size
    // -----------------------------------------------------------------------

    #[test]
    fn size_oz_no_space() {
        assert_eq!(parse_size("12oz"), Some((12.0, "oz".to_owned())));
    }

    #[test]
    fn size_oz_with_space() {
        assert_eq!(parse_size("12 oz"), Some((12.0, "oz".to_owned())));
    }

    #[test]
    fn size_oz_decimal() {
        assert_eq!(parse_size("8.5oz"), Some((8.5, "oz".to_owned())));
    }

    #[test]
    fn size_ml_no_space() {
        assert_eq!(parse_size("355ml"), Some((355.0, "ml".to_owned())));
    }

    #[test]
    fn size_ml_with_space() {
        assert_eq!(parse_size("355 ml"), Some((355.0, "ml".to_owned())));
    }

    #[test]
    fn size_case_insensitive() {
        assert_eq!(parse_size("12OZ"), Some((12.0, "oz".to_owned())));
    }

    #[test]
    fn size_within_complex_title() {
        assert_eq!(parse_size("12oz / 5mg THC"), Some((12.0, "oz".to_owned())));
    }

    #[test]
    fn size_not_present_returns_none() {
        assert!(parse_size("Hi Boy").is_none());
    }

    #[test]
    fn size_default_title_returns_none() {
        assert!(parse_size("Default Title").is_none());
    }
}
