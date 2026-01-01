use std::str::FromStr;

use super::super::tests::{dirs, load_raw_theme};
use super::Version;
use crate::themecfg::{Error, MergeFlags, MergeOptions, Theme, ThemeLoadError};

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
    assert_eq!(Version::CURRENT, Version::V1_0);
}

#[test]
fn test_future_version_rejected() {
    let result = Theme::load(&dirs(), "test-future-version");

    assert!(result.is_err());
    match result {
        Err(Error::FailedToLoadCustomTheme {
            source:
                ThemeLoadError::UnsupportedVersion {
                    requested,
                    nearest,
                    latest,
                },
            ..
        }) => {
            assert_eq!(requested, Version::new(1, 1));
            assert_eq!(nearest, Version::CURRENT);
            assert_eq!(latest, Version::CURRENT);
        }
        _ => panic!("Expected UnsupportedVersion error, got {:?}", result),
    }
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
fn test_v1_version_1_1_rejected_before_deserialization() {
    let result = load_raw_theme("v1-unsupported-version");

    assert!(result.is_err(), "v1 theme with version 1.1 should be rejected");

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    assert!(
        err_msg.contains("1.1") && err_msg.contains("not supported"),
        "Error should indicate version 1.1 is not supported, got: {}",
        err_msg
    );
}

#[test]
fn test_version_merge_flags_unknown() {
    let version = Version::new(99, 0);
    let flags = version.merge_options();
    assert_eq!(flags, MergeFlags::new());
}
