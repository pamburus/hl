use rstest::rstest;

use super::{InnerLevel, Level};

#[rstest]
#[case::debug(InnerLevel::Debug, r#""debug""#)]
#[case::info(InnerLevel::Info, r#""info""#)]
#[case::warning(InnerLevel::Warning, r#""warning""#)]
#[case::error(InnerLevel::Error, r#""error""#)]
fn test_level_serialize_some(#[case] level: InnerLevel, #[case] expected: &str) {
    let level = Level::from(level);
    let json = json::to_string(&level).unwrap();
    assert_eq!(json, expected);
}

#[test]
fn test_level_serialize_none() {
    let level = Level { inner: None };
    let json = json::to_string(&level).unwrap();
    assert_eq!(json, r#""unknown""#);
}

#[test]
fn test_level_deserialize_unknown() {
    let level: Level = json::from_str(r#""unknown""#).unwrap();
    assert_eq!(level.inner, None);
}

#[rstest]
#[case::info(r#""info""#, InnerLevel::Info)]
#[case::debug(r#""debug""#, InnerLevel::Debug)]
#[case::warning(r#""warning""#, InnerLevel::Warning)]
#[case::error(r#""error""#, InnerLevel::Error)]
fn test_level_deserialize_known(#[case] json: &str, #[case] expected: InnerLevel) {
    let level: Level = json::from_str(json).unwrap();
    assert_eq!(level.inner, Some(expected));
}
