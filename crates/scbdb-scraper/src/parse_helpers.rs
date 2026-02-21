//! Internal byte-scanning primitives for dosage and size extraction.
//!
//! All functions operate on pre-lowercased strings unless stated otherwise.
//! This module is `pub(crate)` so that [`crate::parse`] and future sibling
//! modules can share the same low-level routines without exposing them
//! as part of the public API.

/// Maximum byte distance between a numeric value and its THC/CBD label in a
/// variant title or product description.
///
/// 40 bytes covers patterns like:
/// - `"12.5 mg THC"` (compact, typical variant title)
/// - `"5mg Rapid Onset Emulsion THC"` (Adaptaphoria `body_html`: ~30 chars)
/// - `"2MG THC + 6MG CBD"` (Better Than Booze product names)
const MG_LABEL_WINDOW: usize = 40;

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
pub(crate) fn parse_mg_with_label(
    lower: &str,
    label: &str,
    competing_label: Option<&str>,
) -> Option<f64> {
    let mut search_from = 0usize;

    // Try every word-boundary-valid occurrence of the label. Product descriptions
    // may mention the label multiple times (e.g., "Watermelon THC Seltzer...
    // 7.5mg of Delta-9 THC per can") — the dose is near the *second* occurrence,
    // so stopping at the first empty window would silently miss it.
    loop {
        let abs_pos = match lower[search_from..].find(label) {
            None => return None,
            Some(rel_pos) => search_from + rel_pos,
        };

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

        // Advance past this occurrence regardless of outcome so the loop makes
        // forward progress on every iteration.
        search_from = abs_pos + 1;

        if !(before_ok && after_ok) {
            continue; // not a word-boundary match; try the next occurrence
        }

        // Snap window boundaries to valid UTF-8 char boundaries.
        let candidate_start = abs_pos.saturating_sub(MG_LABEL_WINDOW);
        let before_start = (candidate_start..=abs_pos)
            .find(|&i| lower.is_char_boundary(i))
            .unwrap_or(abs_pos);

        let candidate_end = (abs_pos + label.len() + MG_LABEL_WINDOW).min(lower.len());
        let after_end = (candidate_end..=lower.len())
            .find(|&i| lower.is_char_boundary(i))
            .unwrap_or(lower.len());

        // Last mg before the label (closest to it).
        let before_slice = &lower[before_start..abs_pos];

        // If the word immediately preceding the label is a negation (e.g., "No THC"),
        // the compound is explicitly absent and carries no dosage value. Try the
        // next occurrence rather than returning None — the compound may be
        // mentioned again later without negation.
        let last_word_before = before_slice.split_whitespace().next_back().unwrap_or("");
        if matches!(last_word_before, "no" | "non" | "zero" | "without" | "free") {
            continue;
        }

        if let Some(value) = all_mg_values(before_slice).into_iter().last() {
            return Some(value);
        }

        // First mg after the label — but skip if a competing label claims the value.
        let after_slice = &lower[abs_pos + label.len()..after_end];
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

        // No mg value found around this occurrence; try the next one.
    }
}

/// Parses a bare `"Nmg"` or `"N mg"` with no label required.
/// Input must be pre-lowercased.
pub(crate) fn parse_bare_mg(lower: &str) -> Option<f64> {
    extract_mg_value(lower)
}

/// Scans `s` for `Nmg` patterns and calls `on_match` for each one found.
/// If `on_match` returns `Some(v)`, scanning stops and the value is returned.
/// If `on_match` always returns `None`, returns `None` after the full scan.
fn scan_mg_values(s: &str, mut on_match: impl FnMut(f64) -> Option<f64>) -> Option<f64> {
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
                    if let Some(result) = on_match(v) {
                        return Some(result);
                    }
                }
                i = i.saturating_add(2);
                continue;
            }
            i = after_num;
        } else {
            i += 1;
        }
    }
    None
}

/// Returns all `Nmg` values found in `s` in left-to-right order.
/// Input must be pre-lowercased.
fn all_mg_values(s: &str) -> Vec<f64> {
    let mut values = Vec::new();
    scan_mg_values(s, |v| {
        values.push(v);
        None // keep scanning
    });
    values
}

/// Scans `s` for the first occurrence of a number (integer or decimal)
/// optionally followed by whitespace and then `"mg"`. Returns the parsed
/// `f64` value or `None`.
fn extract_mg_value(s: &str) -> Option<f64> {
    scan_mg_values(s, Some)
}

/// Parses a size value followed by `unit` (e.g., `"oz"` or `"ml"`).
/// Input must be pre-lowercased.
pub(crate) fn parse_size_unit(lower: &str, unit: &str) -> Option<(f64, String)> {
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
