//! Low-level string parsing helpers for extracting dosage and size from variant titles.
//!
//! These functions use manual byte scanning rather than `regex` to stay
//! dependency-light. See [`crate::normalize`] for how they compose into full
//! product normalization.
//!
//! Internal scanning primitives live in [`crate::parse_helpers`]; this module
//! exposes the composed, domain-level parsing API.

use crate::parse_helpers::{parse_bare_mg, parse_mg_with_label, parse_size_unit};

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

/// Attempts to extract a THC (or CBD) dosage from raw Shopify `body_html`.
///
/// This is a best-effort fallback for brands like BREZ where `body_html`
/// contains `"3mg micronized THC, 6mg CBD"` but variant titles have no mg
/// values.
///
/// Steps:
/// 1. Strip HTML tags by removing all `<...>` substrings.
/// 2. Decode common HTML entities (`&amp;`, `&lt;`, `&gt;`, `&nbsp;`).
/// 3. Run [`parse_thc_mg`] on the stripped text — return THC if found.
/// 4. Fall back to [`parse_cbd_mg`] if no THC value was found.
///
/// Returns `None` if neither compound yields a parseable dosage.
#[must_use]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn parse_dosage_from_html(html: &str) -> Option<f64> {
    // Strip HTML tags via simple byte scan.
    // Insert a space where each tag was to prevent token concatenation across
    // tag boundaries (e.g., "<b>5mg</b>THC" → "5mg THC" not "5mgTHC").
    let mut stripped = String::with_capacity(html.len());
    let mut inside_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                stripped.push(' ');
            }
            _ if !inside_tag => stripped.push(ch),
            _ => {}
        }
    }

    // Decode common HTML entities.
    let decoded = stripped
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");

    parse_thc_mg(&decoded).or_else(|| parse_cbd_mg(&decoded))
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

#[cfg(test)]
#[path = "parse_test.rs"]
mod tests;
