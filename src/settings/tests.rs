use super::*;

#[test]
fn test_default_settings() {
    let test = |settings: &Settings| {
        assert_eq!(settings.concurrency, None);
        assert_eq!(settings.time_format, "%b %d %T.%3N");
        assert_eq!(settings.time_zone, chrono_tz::UTC);
        assert_eq!(settings.theme, "uni");
    };

    let settings: &'static Settings = Default::default();
    test(settings);
    test(&Settings::default());
}

#[test]
fn test_load_settings_k8s() {
    let settings = Settings::load([SourceFile::new("etc/defaults/config-k8s.toml").into()]).unwrap();
    assert_eq!(
        settings.fields.predefined.time,
        TimeField(Field {
            names: vec!["ts".into()],
            show: FieldShowOption::Auto,
        })
    );
    assert_eq!(settings.time_format, "%b %d %T.%3N");
    assert_eq!(settings.time_zone, chrono_tz::UTC);
    assert_eq!(settings.theme, "uni");
}

#[test]
fn test_unknown_level_values() {
    let variant = RawLevelFieldVariant {
        names: vec!["level".into()],
        values: vec![
            (InfallibleLevel::Valid(Level::Info), vec!["info".into()]),
            (InfallibleLevel::Invalid("unknown".into()), vec!["unknown".into()]),
        ]
        .into_iter()
        .collect(),
        level: None,
    };

    assert_eq!(
        variant.resolve(),
        Some(LevelFieldVariant {
            names: vec!["level".into()],
            values: vec![(Level::Info, vec!["info".into()])].into_iter().collect(),
            level: None,
        })
    );
}

#[test]
fn test_unknown_level_main() {
    let variant = RawLevelFieldVariant {
        names: vec!["level".into()],
        values: vec![(InfallibleLevel::Valid(Level::Info), vec!["info".into()])]
            .into_iter()
            .collect(),
        level: Some(InfallibleLevel::Invalid("unknown".into())),
    };

    assert_eq!(variant.resolve(), None);
}

#[test]
fn test_unknown_level_all_unknown() {
    let variant = RawLevelFieldVariant {
        names: vec!["level".into()],
        values: vec![(InfallibleLevel::Invalid("unknown".into()), vec!["unknown".into()])]
            .into_iter()
            .collect(),
        level: Some(InfallibleLevel::Valid(Level::Info)),
    };

    assert_eq!(variant.resolve(), None);
}

#[test]
fn test_csl() {
    let csl = ListOrCommaSeparatedList::from(vec!["a", "b", "c"]);
    assert_eq!(csl.deref(), vec!["a", "b", "c"]);
    assert_eq!(Vec::from(csl), vec!["a", "b", "c"]);

    let csl: ListOrCommaSeparatedList<String> = serde_plain::from_str("a,b,c").unwrap();
    assert_eq!(csl.deref(), vec!["a", "b", "c"]);
    assert_eq!(Vec::from(csl), vec!["a", "b", "c"]);

    let csl: ListOrCommaSeparatedList<String> = serde_plain::from_str("").unwrap();
    assert_eq!(csl.deref(), Vec::<String>::new());

    let csl = serde_json::from_str::<ListOrCommaSeparatedList<String>>(r#""a,b,c""#).unwrap();
    assert_eq!(csl.deref(), vec!["a", "b", "c"]);

    let csl = serde_json::from_str::<ListOrCommaSeparatedList<String>>(r#""""#).unwrap();
    assert_eq!(csl.deref(), Vec::<String>::new());

    let res = serde_json::from_str::<ListOrCommaSeparatedList<String>>(r#"12"#);
    assert!(res.is_err());
}

#[test]
fn test_ascii_mode_opt() {
    // Default value should be Auto
    assert_eq!(AsciiModeOpt::default(), AsciiModeOpt::Auto);
}

#[test]
fn test_ascii_mode_opt_resolve() {
    // Test resolve with utf8_supported = true
    assert_eq!(AsciiModeOpt::Auto.resolve(true), AsciiMode::Off);
    assert_eq!(AsciiModeOpt::Always.resolve(true), AsciiMode::On);
    assert_eq!(AsciiModeOpt::Never.resolve(true), AsciiMode::Off);

    // Test resolve with utf8_supported = false
    assert_eq!(AsciiModeOpt::Auto.resolve(false), AsciiMode::On);
    assert_eq!(AsciiModeOpt::Always.resolve(false), AsciiMode::On);
    assert_eq!(AsciiModeOpt::Never.resolve(false), AsciiMode::Off);
}

#[test]
fn test_display_variant_uniform() {
    let uniform = DisplayVariant::Uniform("test".to_string());

    // Uniform variant should return the same string regardless of mode
    assert_eq!(uniform.resolve(AsciiMode::On), "test");
    assert_eq!(uniform.resolve(AsciiMode::Off), "test");
}

#[test]
fn test_display_variant_selective() {
    let selective = DisplayVariant::Selective {
        ascii: "ascii".to_string(),
        unicode: "unicode".to_string(),
    };

    // Selective variant should return the appropriate string based on mode
    assert_eq!(selective.resolve(AsciiMode::On), "ascii");
    assert_eq!(selective.resolve(AsciiMode::Off), "unicode");
}

#[test]
fn test_display_variant_from_string() {
    let from_string = DisplayVariant::from("test".to_string());
    assert!(matches!(from_string, DisplayVariant::Uniform(_)));
    assert_eq!(from_string.resolve(AsciiMode::Off), "test");
}

#[test]
fn test_display_variant_from_str() {
    let from_str = DisplayVariant::from("test");
    assert_eq!(from_str, DisplayVariant::Uniform("test".to_string()));
}

#[test]
fn test_display_variant_resolve() {
    // Test with uniform variant
    let uniform = DisplayVariant::Uniform("test".to_string());
    assert_eq!(uniform.resolve(AsciiMode::On), "test");
    assert_eq!(uniform.resolve(AsciiMode::Off), "test");

    // Test with selective variant
    let selective = DisplayVariant::Selective {
        ascii: "ascii".to_string(),
        unicode: "unicode".to_string(),
    };
    assert_eq!(selective.resolve(AsciiMode::On), "ascii");
    assert_eq!(selective.resolve(AsciiMode::Off), "unicode");
}

#[test]
fn test_punctuation_resolve() {
    // Use Punctuation::sample instead of Default::default to avoid dependency on default config
    let mut punctuation = Punctuation::sample();

    // Set up selective variants for multiple punctuation elements
    punctuation.input_number_right_separator = DisplayVariant::Selective {
        ascii: " | ".to_string(),
        unicode: " │ ".to_string(),
    };
    punctuation.source_location_separator = DisplayVariant::Selective {
        ascii: "-> ".to_string(),
        unicode: "→ ".to_string(),
    };
    punctuation.array_separator = DisplayVariant::Selective {
        ascii: ", ".to_string(),
        unicode: "· ".to_string(),
    };
    punctuation.hidden_fields_indicator = DisplayVariant::Selective {
        ascii: "...".to_string(),
        unicode: "…".to_string(),
    };

    // Test with direct resolve calls
    assert_eq!(punctuation.input_number_right_separator.resolve(AsciiMode::On), " | ");
    assert_eq!(punctuation.input_number_right_separator.resolve(AsciiMode::Off), " │ ");
    assert_eq!(punctuation.source_location_separator.resolve(AsciiMode::On), "-> ");
    assert_eq!(punctuation.source_location_separator.resolve(AsciiMode::Off), "→ ");
    assert_eq!(punctuation.array_separator.resolve(AsciiMode::On), ", ");
    assert_eq!(punctuation.array_separator.resolve(AsciiMode::Off), "· ");
    assert_eq!(punctuation.hidden_fields_indicator.resolve(AsciiMode::On), "...");
    assert_eq!(punctuation.hidden_fields_indicator.resolve(AsciiMode::Off), "…");

    // Test ASCII mode through Punctuation::resolve
    let resolved_ascii = punctuation.resolve(AsciiMode::On);
    let resolved_utf8 = punctuation.resolve(AsciiMode::Off);

    // Verify ASCII version of resolved punctuation
    assert_eq!(resolved_ascii.input_number_right_separator, " | ");
    assert_eq!(resolved_ascii.source_location_separator, "-> ");
    assert_eq!(resolved_ascii.array_separator, ", ");
    assert_eq!(resolved_ascii.hidden_fields_indicator, "...");

    // Verify Unicode version of resolved punctuation
    assert_eq!(resolved_utf8.input_number_right_separator, " │ ");
    assert_eq!(resolved_utf8.source_location_separator, "→ ");
    assert_eq!(resolved_utf8.array_separator, "· ");
    assert_eq!(resolved_utf8.hidden_fields_indicator, "…");

    // Test that all fields are correctly resolved
    for (ascii_val, utf8_val) in [
        (
            resolved_ascii.input_number_right_separator.as_str(),
            resolved_utf8.input_number_right_separator.as_str(),
        ),
        (
            resolved_ascii.source_location_separator.as_str(),
            resolved_utf8.source_location_separator.as_str(),
        ),
    ] {
        assert_ne!(ascii_val, utf8_val, "ASCII and Unicode values should be different");
    }
}

#[test]
fn test_expansion_options() {
    let mut profiles = ExpansionProfiles::default();
    profiles.low.thresholds.global = Some(1);
    profiles.low.thresholds.cumulative = Some(2);
    profiles.low.thresholds.message = Some(3);
    profiles.medium.thresholds.global = Some(4);
    profiles.medium.thresholds.field = Some(5);
    profiles.high.thresholds.global = Some(6);
    profiles.high.thresholds.cumulative = Some(7);
    let xo = |mode| ExpansionOptions {
        mode,
        profiles: profiles.clone(),
    };
    assert_eq!(xo(None).profile(), None);
    assert_eq!(xo(Some(ExpansionMode::Never)).profile(), Some(&ExpansionProfile::NEVER));
    assert_eq!(
        xo(Some(ExpansionMode::Inline)).profile(),
        Some(&ExpansionProfile::INLINE)
    );
    assert_eq!(xo(Some(ExpansionMode::Low)).profile(), Some(&profiles.low));
    assert_eq!(xo(Some(ExpansionMode::Medium)).profile(), Some(&profiles.medium));
    assert_eq!(xo(Some(ExpansionMode::High)).profile(), Some(&profiles.high));
    assert_eq!(
        xo(Some(ExpansionMode::Always)).profile(),
        Some(&ExpansionProfile::ALWAYS)
    );
}
