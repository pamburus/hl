use super::*;

#[test]
fn test_relaxed_level_from_conversion() {
    let relaxed = RelaxedLevel(Level::Info);
    let level: Level = relaxed.into();
    assert_eq!(level, Level::Info);

    let relaxed = RelaxedLevel(Level::Error);
    let level: Level = Level::from(relaxed);
    assert_eq!(level, Level::Error);
}

#[test]
fn test_relaxed_level_deref() {
    let relaxed = RelaxedLevel(Level::Warning);
    assert_eq!(*relaxed, Level::Warning);
    assert_eq!(relaxed.deref(), &Level::Warning);
}

#[test]
fn test_relaxed_level_try_from_str() {
    // Test case-insensitive parsing
    assert_eq!(RelaxedLevel::try_from("info").unwrap().0, Level::Info);
    assert_eq!(RelaxedLevel::try_from("INFO").unwrap().0, Level::Info);
    assert_eq!(RelaxedLevel::try_from("Info").unwrap().0, Level::Info);

    assert_eq!(RelaxedLevel::try_from("error").unwrap().0, Level::Error);
    assert_eq!(RelaxedLevel::try_from("ERROR").unwrap().0, Level::Error);

    assert_eq!(RelaxedLevel::try_from("warn").unwrap().0, Level::Warning);
    assert_eq!(RelaxedLevel::try_from("warning").unwrap().0, Level::Warning);

    // Test invalid input
    assert!(RelaxedLevel::try_from("invalid").is_err());
}

#[test]
fn test_level_value_parser() {
    let _parser = LevelValueParser;

    // Test that alternate values are available
    let alternate_values = LevelValueParser::alternate_values();
    assert!(!alternate_values.is_empty());

    // Verify some expected alternate values exist
    let has_warning = alternate_values
        .iter()
        .any(|(level, values)| *level == Level::Warning && values.contains(&"warning"));
    assert!(has_warning, "Should have 'warning' as alternate for Warn level");
}
