use super::*;

#[test]
fn condition_parse_os_macos() {
    let cond: Condition = "os:macos".parse().expect("failed to parse");
    assert_eq!(cond, Condition::Os(OsCondition::MacOS));
    assert_eq!(cond.to_string(), "os:macos");
}

#[test]
fn condition_parse_os_linux() {
    let cond: Condition = "os:linux".parse().expect("failed to parse");
    assert_eq!(cond, Condition::Os(OsCondition::Linux));
    assert_eq!(cond.to_string(), "os:linux");
}

#[test]
fn condition_parse_os_windows() {
    let cond: Condition = "os:windows".parse().expect("failed to parse");
    assert_eq!(cond, Condition::Os(OsCondition::Windows));
    assert_eq!(cond.to_string(), "os:windows");
}

#[test]
fn condition_parse_os_unix() {
    let cond: Condition = "os:unix".parse().expect("failed to parse");
    assert_eq!(cond, Condition::Os(OsCondition::Unix));
    assert_eq!(cond.to_string(), "os:unix");
}

#[test]
fn condition_parse_mode_view() {
    let cond: Condition = "mode:view".parse().expect("failed to parse");
    assert_eq!(cond, Condition::Mode(ModeCondition::View));
    assert_eq!(cond.to_string(), "mode:view");
}

#[test]
fn condition_parse_mode_follow() {
    let cond: Condition = "mode:follow".parse().expect("failed to parse");
    assert_eq!(cond, Condition::Mode(ModeCondition::Follow));
    assert_eq!(cond.to_string(), "mode:follow");
}

#[test]
fn condition_parse_negation() {
    let cond: Condition = "!mode:follow".parse().expect("failed to parse");
    assert_eq!(cond, Condition::Not(Box::new(Condition::Mode(ModeCondition::Follow))));
    assert_eq!(cond.to_string(), "!mode:follow");
}

#[test]
fn condition_parse_negation_with_spaces() {
    let cond: Condition = "! mode:view".parse().expect("failed to parse");
    assert_eq!(cond, Condition::Not(Box::new(Condition::Mode(ModeCondition::View))));
}

#[test]
fn condition_parse_with_whitespace() {
    let cond: Condition = "  os:macos  ".parse().expect("failed to parse");
    assert_eq!(cond, Condition::Os(OsCondition::MacOS));
}

#[test]
fn condition_parse_unknown_os() {
    let result: Result<Condition, _> = "os:freebsd".parse();
    assert!(matches!(result, Err(ConditionError::UnknownOs(_))));
}

#[test]
fn condition_parse_unknown_mode() {
    let result: Result<Condition, _> = "mode:stream".parse();
    assert!(matches!(result, Err(ConditionError::UnknownMode(_))));
}

#[test]
fn condition_parse_unknown_prefix() {
    let result: Result<Condition, _> = "arch:x86_64".parse();
    assert!(matches!(result, Err(ConditionError::UnknownPrefix(_))));
}

#[test]
fn condition_parse_missing_prefix() {
    let result: Result<Condition, _> = "macos".parse();
    assert!(matches!(result, Err(ConditionError::MissingPrefix(_))));
}

#[test]
fn condition_matches_os() {
    #[cfg(target_os = "macos")]
    {
        let cond = Condition::Os(OsCondition::MacOS);
        assert!(cond.matches(&ConditionContext::with_mode(ConditionMode::View)));
        assert!(cond.matches(&ConditionContext::with_mode(ConditionMode::Follow)));
    }

    #[cfg(target_os = "linux")]
    {
        let cond = Condition::Os(OsCondition::Linux);
        assert!(cond.matches(&ConditionContext::with_mode(ConditionMode::View)));
        assert!(cond.matches(&ConditionContext::with_mode(ConditionMode::Follow)));
    }
}

#[test]
fn condition_matches_unix() {
    #[cfg(unix)]
    {
        let cond = Condition::Os(OsCondition::Unix);
        assert!(cond.matches(&ConditionContext::with_mode(ConditionMode::View)));
        assert!(cond.matches(&ConditionContext::with_mode(ConditionMode::Follow)));
    }

    #[cfg(not(unix))]
    {
        let cond = Condition::Os(OsCondition::Unix);
        assert!(!cond.matches(&ConditionContext::with_mode(ConditionMode::View)));
        assert!(!cond.matches(&ConditionContext::with_mode(ConditionMode::Follow)));
    }
}

#[test]
fn condition_matches_mode_view() {
    let cond = Condition::Mode(ModeCondition::View);
    assert!(cond.matches(&ConditionContext::with_mode(ConditionMode::View)));
    assert!(!cond.matches(&ConditionContext::with_mode(ConditionMode::Follow)));
}

#[test]
fn condition_matches_mode_follow() {
    let cond = Condition::Mode(ModeCondition::Follow);
    assert!(!cond.matches(&ConditionContext::with_mode(ConditionMode::View)));
    assert!(cond.matches(&ConditionContext::with_mode(ConditionMode::Follow)));
}

#[test]
fn condition_matches_negation() {
    let cond = Condition::Not(Box::new(Condition::Mode(ModeCondition::Follow)));
    assert!(cond.matches(&ConditionContext::with_mode(ConditionMode::View)));
    assert!(!cond.matches(&ConditionContext::with_mode(ConditionMode::Follow)));
}

#[test]
fn condition_matches_mode_without_context() {
    // When no mode is provided in context, mode conditions evaluate to false.
    let cond = Condition::Mode(ModeCondition::View);
    assert!(!cond.matches(&ConditionContext::default()));

    // Negation of a mode condition also evaluates to true when no mode context.
    let cond = Condition::Not(Box::new(Condition::Mode(ModeCondition::View)));
    assert!(cond.matches(&ConditionContext::default()));
}

#[test]
fn condition_deserialize_from_toml() {
    #[derive(Deserialize)]
    struct TestConfig {
        r#if: Condition,
    }

    let toml = r#"if = "os:macos""#;
    let config: TestConfig = toml::from_str(toml).expect("failed to parse");
    assert_eq!(config.r#if, Condition::Os(OsCondition::MacOS));

    let toml = r#"if = "!mode:follow""#;
    let config: TestConfig = toml::from_str(toml).expect("failed to parse");
    assert_eq!(
        config.r#if,
        Condition::Not(Box::new(Condition::Mode(ModeCondition::Follow)))
    );
}

#[test]
fn condition_error_display() {
    let err = ConditionError::UnknownPrefix("arch".to_string());
    assert!(err.to_string().contains("arch"));
    assert!(err.to_string().contains("valid"));

    let err = ConditionError::MissingPrefix("macos".to_string());
    assert!(err.to_string().contains("macos"));

    let err = ConditionError::UnknownOs("freebsd".to_string());
    assert!(err.to_string().contains("freebsd"));

    let err = ConditionError::UnknownMode("stream".to_string());
    assert!(err.to_string().contains("stream"));
}
