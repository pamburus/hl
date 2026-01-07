use std::str::FromStr;

use rstest::rstest;

use super::super::tests::load_raw_theme;
use super::Version;
use crate::themecfg::{Format, MergeFlags, MergeOptions, Theme};

#[test]
fn test_theme_version_parsing() {
    // Valid versions
    assert_eq!(Version::from_str("1.0").unwrap(), Version::new(1, 0));
    assert_eq!(Version::from_str("1.10").unwrap(), Version::new(1, 10));
    assert_eq!(Version::from_str("2.123").unwrap(), Version::new(2, 123));
    assert_eq!(Version::from_str("0.0").unwrap(), Version::new(0, 0));

    // Invalid versions - leading zeros
    assert!(Version::from_str("1.01").is_err());
    assert!(Version::from_str("01.0").is_err());
    assert!(Version::from_str("01.01").is_err());

    // Invalid versions - missing components
    assert!(Version::from_str("1").is_err());
    assert!(Version::from_str("1.").is_err());
    assert!(Version::from_str(".1").is_err());

    // Invalid versions - not numbers
    assert!(Version::from_str("1.x").is_err());
    assert!(Version::from_str("x.1").is_err());
    assert!(Version::from_str("a.b").is_err());

    // Invalid versions - extra components
    assert!(Version::from_str("1.0.0").is_err());
}

#[test]
fn test_theme_version_display() {
    assert_eq!(Version::new(1, 0).to_string(), "1.0");
    assert_eq!(Version::new(1, 10).to_string(), "1.10");
    assert_eq!(Version::new(2, 123).to_string(), "2.123");
    assert_eq!(Version::new(0, 0).to_string(), "0.0");
}

#[test]
fn test_theme_version_compatibility() {
    let v1_0 = Version::new(1, 0);
    let v1_1 = Version::new(1, 1);
    let v1_2 = Version::new(1, 2);
    let v2_0 = Version::new(2, 0);

    // Same version is compatible
    assert!(v1_0.is_compatible_with(&v1_0));
    assert!(v1_1.is_compatible_with(&v1_1));

    // Older minor version is compatible
    assert!(v1_0.is_compatible_with(&v1_1));
    assert!(v1_0.is_compatible_with(&v1_2));
    assert!(v1_1.is_compatible_with(&v1_2));

    // Newer minor version is not compatible
    assert!(!v1_1.is_compatible_with(&v1_0));
    assert!(!v1_2.is_compatible_with(&v1_0));
    assert!(!v1_2.is_compatible_with(&v1_1));

    // Different major version is not compatible
    assert!(!v2_0.is_compatible_with(&v1_0));
    assert!(!v1_0.is_compatible_with(&v2_0));
}

#[test]
fn test_theme_version_serde() {
    // Deserialize
    let version: Version = serde_json::from_str(r#""1.0""#).unwrap();
    assert_eq!(version, Version::new(1, 0));

    let version: Version = serde_json::from_str(r#""2.15""#).unwrap();
    assert_eq!(version, Version::new(2, 15));

    // Serialize
    let version = Version::new(1, 0);
    let json = serde_json::to_string(&version).unwrap();
    assert_eq!(json, r#""1.0""#);

    let version = Version::new(2, 15);
    let json = serde_json::to_string(&version).unwrap();
    assert_eq!(json, r#""2.15""#);

    // Invalid formats should fail
    assert!(serde_json::from_str::<Version>(r#""1.01""#).is_err());
    assert!(serde_json::from_str::<Version>(r#""1""#).is_err());
    assert!(serde_json::from_str::<Version>(r#"1"#).is_err());
}

#[test]
fn test_theme_version_constants() {
    assert_eq!(Version::V0_0, Version::new(0, 0));
    assert_eq!(Version::V1_0, Version::new(1, 0));
}

#[test]
fn test_v0_version_0_1_rejected() {
    let result = load_raw_theme("v0-invalid-version");

    assert!(result.is_err(), "v0 theme with version 0.1 should be rejected");

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    assert!(
        err_msg.contains("0.1") && err_msg.contains("not supported"),
        "Error should indicate version 0.1 is not supported, got: {}",
        err_msg
    );
}

#[test]
fn test_future_version_rejected_before_deserialization() {
    let version = format!("{}.{}", Version::CURRENT.major, Version::CURRENT.minor + 1);
    let data = format!(r#"version = "{}""#, &version);
    let result = Theme::from_buf(data.as_bytes(), Format::Toml);

    assert!(result.is_err(), "theme with version {} should be rejected", version);

    let err = result.unwrap_err();
    let msg = err.to_string();

    assert!(
        msg.contains(&version) && msg.contains("not supported"),
        "error should indicate version {} is not supported, got: {}",
        version,
        msg
    );
}

#[test]
fn test_version_merge_flags_unknown() {
    let version = Version::new(99, 0);
    let flags = version.merge_options();
    assert_eq!(flags, MergeFlags::new());
}

#[test]
fn test_version_parse_empty_string() {
    assert_eq!(Version::parse(""), None);
}

#[test]
fn test_version_must_parse_valid() {
    let version = Version::must_parse("1.0");
    assert_eq!(version, Version::new(1, 0));
}

#[test]
fn test_version_equals() {
    let v1 = Version::new(1, 0);
    let v2 = Version::new(1, 0);
    let v3 = Version::new(1, 1);

    assert!(v1.equals(&v2));
    assert!(!v1.equals(&v3));
}

#[rstest]
#[case("01.0")]
#[case("1.01")]
#[case("0.01")]
#[case("00.0")]
#[case("0.00")]
#[case(".0")]
#[case("1.")]
fn test_version_const_parse_leading_zeros(#[case] input: &str) {
    assert_eq!(Version::parse(input), None);
}

#[rstest]
#[case("0.0", Version::new(0, 0))]
#[case("0.1", Version::new(0, 1))]
#[case("1.0", Version::new(1, 0))]
fn test_version_parse_single_zero_valid(#[case] input: &str, #[case] expected: Version) {
    assert_eq!(Version::parse(input), Some(expected));
}

#[rstest]
#[case("01.0")]
#[case("1.01")]
fn test_version_parse_runtime_leading_zeros(#[case] input: &str) {
    let version_str = String::from(input);
    assert_eq!(Version::parse(&version_str), None);
}
