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

// I2: word boundary tests
#[test]
fn thc_mg_negation_context_returns_none() {
    assert_eq!(parse_thc_mg("No THC 5mg CBD"), None);
}

#[test]
fn thc_mg_thcv_substring_returns_none() {
    assert_eq!(parse_thc_mg("5mg THCv"), None);
}

#[test]
fn thc_mg_non_ascii_before_label_no_panic() {
    assert_eq!(parse_thc_mg("café 5mg THC"), Some(5.0));
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
