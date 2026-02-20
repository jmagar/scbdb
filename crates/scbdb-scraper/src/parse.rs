//! Low-level string parsing helpers for extracting dosage and size from variant titles.
//!
//! These functions use manual byte scanning rather than `regex` to stay
//! dependency-light. See [`crate::normalize`] for how they compose into full
//! product normalization.

/// Maximum byte distance between a numeric value and its THC/CBD label in a variant title.
/// Covers patterns like "12.5 mg THC" with surrounding spaces and punctuation.
const MG_LABEL_WINDOW: usize = 20;

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
    parse_mg_with_label(&lower, "thc", Some("cbd")).or_else(|| {
        // Bare "Nmg" with no CBD or THC-related label: attribute to THC.
        // Suppress if "cbd" or any "thc" substring (e.g. "thcv", "thca") is present,
        // since the bare mg value likely belongs to the labeled compound.
        if lower.contains("cbd") || lower.contains("thc") {
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
    parse_mg_with_label(&lower, "cbd", Some("thc"))
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
/// `competing_label` is an optional label for a different compound (e.g., `"cbd"`
/// when searching for `"thc"`). When an mg value in the after-slice is immediately
/// followed by the competing label, the value is attributed to that compound
/// instead and skipped.
///
/// Strategy: split the window around the label.
/// - **Before label** (e.g., `"6mg CBD"`): collect all mg values before the
///   label and return the *last* one (closest to the label).
/// - **After label** (e.g., `"CBD 6mg"`): return the first mg value after
///   the label, unless a competing label claims it.
///
/// This correctly handles combined titles like `"3mg THC, 6mg CBD"` where
/// the before-label scan finds both 3mg and 6mg but returns 6mg (last/closest).
fn parse_mg_with_label(lower: &str, label: &str, competing_label: Option<&str>) -> Option<f64> {
    let mut search_from = 0usize;

    let label_pos = loop {
        match lower[search_from..].find(label) {
            None => return None,
            Some(rel_pos) => {
                let abs_pos = search_from + rel_pos;
                let before_ok = abs_pos == 0
                    || !lower[..abs_pos]
                        .chars()
                        .last()
                        .is_some_and(char::is_alphanumeric);
                let after_end_pos = abs_pos + label.len();
                let after_ok = after_end_pos >= lower.len()
                    || !lower[after_end_pos..]
                        .chars()
                        .next()
                        .is_some_and(char::is_alphanumeric);
                if before_ok && after_ok {
                    break abs_pos;
                }
                search_from = abs_pos + 1;
            }
        }
    };

    // Snap window boundaries to valid UTF-8 char boundaries.
    let candidate_start = label_pos.saturating_sub(MG_LABEL_WINDOW);
    let before_start = (candidate_start..=label_pos)
        .find(|&i| lower.is_char_boundary(i))
        .unwrap_or(label_pos);

    let candidate_end = (label_pos + label.len() + MG_LABEL_WINDOW).min(lower.len());
    let after_end = (0..=candidate_end)
        .rev()
        .find(|&i| lower.is_char_boundary(i))
        .unwrap_or(lower.len());

    // Last mg before the label (closest to it).
    let before_slice = &lower[before_start..label_pos];
    if let Some(value) = all_mg_values(before_slice).into_iter().last() {
        return Some(value);
    }

    // First mg after the label — but skip if a competing label claims the value.
    let after_slice = &lower[label_pos + label.len()..after_end];
    if let Some(value) = extract_mg_value(after_slice) {
        // The value is dominated by the competitor only when the competing label
        // appears *before* the mg value in the after-slice. This preserves correct
        // attribution for titles like "THC 5mg, CBD 3mg" where CBD appears after
        // the THC mg value and should not suppress the THC reading.
        //
        // Example: "thc 5mg cbd" → after "thc": " 5mg cbd" → CBD at pos 5, mg
        // starts at pos 1 → NOT dominated → 5mg belongs to THC (correct).
        // Example: "thc cbd 5mg" → after "thc": " cbd 5mg" → CBD at pos 1, mg
        // starts at pos 5 → dominated → skip (5mg belongs to CBD).
        let dominated_by_competitor = competing_label.is_some_and(|comp| {
            after_slice.find(comp).is_some_and(|comp_pos| {
                let mg_start = after_slice
                    .bytes()
                    .position(|b| b.is_ascii_digit() || b == b'.');
                mg_start.is_some_and(|mp| comp_pos < mp)
            })
        });
        if !dominated_by_competitor {
            return Some(value);
        }
    }

    None
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
#[path = "parse_test.rs"]
mod tests;
