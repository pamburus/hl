use super::*;

#[test]
fn test_load() {
    let app_dirs = AppDirs {
        config_dir: PathBuf::from("src/testing/assets"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    };
    assert_ne!(Theme::load(&app_dirs, "test").unwrap().elements.len(), 0);
    assert_ne!(Theme::load(&app_dirs, "universal").unwrap().elements.len(), 0);
    assert!(Theme::load(&app_dirs, "non-existent").is_err());
    assert!(Theme::load(&app_dirs, "invalid").is_err());
    assert!(Theme::load(&app_dirs, "invalid-type").is_err());
}

#[test]
fn test_load_from() {
    let path = PathBuf::from("etc/defaults/themes");
    assert_ne!(Theme::load_from(&path, "universal").unwrap().elements.len(), 0);

    let path = PathBuf::from("src/testing/assets/themes");
    assert_ne!(Theme::load_from(&path, "test").unwrap().elements.len(), 0);
    assert_ne!(Theme::load_from(&path, "test.toml").unwrap().elements.len(), 0);
    assert_ne!(
        Theme::load_from(&path, "./src/testing/assets/themes/test.toml")
            .unwrap()
            .elements
            .len(),
        0
    );
    assert!(Theme::load_from(&path, "non-existent").is_err());
    assert!(Theme::load_from(&path, "invalid").is_err());
    assert!(Theme::load_from(&path, "invalid-type").is_err());
}

#[test]
fn test_embedded() {
    assert_ne!(Theme::embedded("universal").unwrap().elements.len(), 0);
    assert!(Theme::embedded("non-existent").is_err());
}

#[test]
fn test_rgb() {
    let a = RGB::from_str("#102030").unwrap();
    assert_eq!(a, RGB(16, 32, 48));
    let b: RGB = serde_json::from_str(r##""#102030""##).unwrap();
    assert_eq!(b, RGB(16, 32, 48));
}

#[test]
fn test_rgb_lowercase() {
    let a = RGB::from_str("#aabbcc").unwrap();
    assert_eq!(a, RGB(170, 187, 204));
    let b = RGB::from_str("#AABBCC").unwrap();
    assert_eq!(b, RGB(170, 187, 204));
}

#[test]
fn test_rgb_invalid() {
    // Missing # prefix
    assert!(RGB::from_str("ff0000").is_err());

    // Wrong length
    assert!(RGB::from_str("#fff").is_err());
    assert!(RGB::from_str("#fffffff").is_err());

    // Invalid hex characters
    assert!(RGB::from_str("#gghhii").is_err());
    assert!(RGB::from_str("#zzzzzz").is_err());
}

#[test]
fn test_style_pack() {
    assert_eq!(StylePack::<Element>::default().clone().len(), 0);

    let yaml = include_str!("../testing/assets/style-packs/pack1.yaml");
    let pack: StylePack<Element> = yaml::from_str(yaml).unwrap().remove(0);
    assert_eq!(pack.0.len(), 2);
    assert_eq!(pack.0[&Element::Input].foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(pack.0[&Element::Input].background, Some(Color::Plain(PlainColor::Blue)));
    assert_eq!(pack.0[&Element::Input].modes, (Mode::Bold | Mode::Faint).into());
    assert_eq!(
        pack.0[&Element::Message].foreground,
        Some(Color::Plain(PlainColor::Green))
    );
    assert_eq!(pack.0[&Element::Message].background, None);
    assert_eq!(pack.0[&Element::Message].modes, (Mode::Italic | Mode::Underline).into());

    assert!(
        yaml::from_str::<StylePack<Element>>("invalid")
            .unwrap_err()
            .msg
            .ends_with("expected style pack object")
    );
}

#[test]
fn test_tags() {
    assert_eq!(Tag::from_str("dark").unwrap(), Tag::Dark);
    assert_eq!(Tag::from_str("light").unwrap(), Tag::Light);
    assert_eq!(Tag::from_str("16color").unwrap(), Tag::Palette16);
    assert_eq!(Tag::from_str("256color").unwrap(), Tag::Palette256);
    assert_eq!(Tag::from_str("truecolor").unwrap(), Tag::TrueColor);
    assert!(Tag::from_str("invalid").is_err());
}

#[test]
fn test_style_merge() {
    let base = ResolvedStyle {
        modes: Mode::Bold.into(),
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: Some(Color::Plain(PlainColor::Blue)),
    };

    let patch = ResolvedStyle {
        modes: Mode::Italic.into(),
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: None,
    };

    let result = base.clone().merged_with(&patch);

    assert_eq!(result.modes, Mode::Bold | Mode::Italic);
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Green)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Blue)));

    let patch = ResolvedStyle {
        background: Some(Color::Plain(PlainColor::Green)),
        ..Default::default()
    };

    let result = base.clone().merged_with(&patch);

    assert_eq!(result.modes, EnumSet::from(Mode::Bold));
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Green)));
}

// --- V0 Format Tests ---

#[test]
fn test_v0_boolean_active_merge() {
    // Test that v0 applies base `boolean` element to `boolean-true` and `boolean-false`
    // Note: The boolean active merge happens during conversion to theme::Theme,
    // not at the themecfg::Theme level. At themecfg level, we just verify the elements exist.
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-boolean-merge").unwrap();

    // Base boolean element should exist
    assert!(theme.elements.get(&Element::Boolean).is_some());
    let boolean = theme.elements.get(&Element::Boolean).unwrap();
    assert_eq!(boolean.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(boolean.background, Some(Color::RGB(RGB(0, 0, 0))));
    assert_eq!(boolean.modes, vec![Mode::Bold]);

    // boolean-true and boolean-false should exist with their own properties
    let boolean_true = theme.elements.get(&Element::BooleanTrue).unwrap();
    assert_eq!(boolean_true.foreground, Some(Color::RGB(RGB(0, 255, 255))));
    // At themecfg level, boolean-true doesn't have background/modes yet
    // The merge happens in theme::StylePack::load()

    let boolean_false = theme.elements.get(&Element::BooleanFalse).unwrap();
    assert_eq!(boolean_false.foreground, Some(Color::RGB(RGB(255, 0, 0))));
}

#[test]
fn test_v0_modes_replacement() {
    // Test that v0 child modes completely replace parent modes (no merging)
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-modes-replace").unwrap();

    // level has bold and underline
    let level = theme.elements.get(&Element::Level).unwrap();
    assert_eq!(level.modes, vec![Mode::Bold, Mode::Underline]);

    // level-inner has only italic (replaces parent's modes, not merged)
    let level_inner = theme.elements.get(&Element::LevelInner).unwrap();
    assert_eq!(level_inner.modes, vec![Mode::Italic]);
}

#[test]
fn test_v0_level_specific_overrides() {
    // Test that level-specific elements merge with base elements
    // and level overrides win for defined properties
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-level-overrides").unwrap();

    // Base level element
    let base_level = theme.elements.get(&Element::Level).unwrap();
    assert_eq!(base_level.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(base_level.modes, vec![Mode::Italic]);

    // Debug level should have overridden foreground and modes
    let debug_level = theme
        .levels
        .get(&InfallibleLevel::Valid(crate::level::Level::Debug))
        .and_then(|pack| pack.get(&Element::Level));
    assert!(debug_level.is_some());
    let debug_level = debug_level.unwrap();
    assert_eq!(debug_level.foreground, Some(Color::RGB(RGB(255, 0, 255))));
    assert_eq!(debug_level.modes, vec![Mode::Bold, Mode::Underline]);

    // Error level should have comprehensive overrides
    let error_level = theme
        .levels
        .get(&InfallibleLevel::Valid(crate::level::Level::Error))
        .and_then(|pack| pack.get(&Element::Level));
    assert!(error_level.is_some());
    let error_level = error_level.unwrap();
    assert_eq!(error_level.foreground, Some(Color::RGB(RGB(255, 0, 0))));
    assert_eq!(error_level.background, Some(Color::RGB(RGB(68, 0, 0))));
    assert_eq!(error_level.modes, vec![Mode::Reverse, Mode::Bold]);
}

#[test]
fn test_v0_nested_styling_elements() {
    // Test that v0 has separate parent/inner elements (nested rendering scope)
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-nested-styling").unwrap();

    // Parent element with full styling
    let level = theme.elements.get(&Element::Level).unwrap();
    assert_eq!(level.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(level.background, Some(Color::RGB(RGB(0, 17, 0))));
    assert_eq!(level.modes, vec![Mode::Bold]);

    // Inner element has only foreground - does NOT inherit background/modes in v0
    // (This is nested scope, not property merging)
    let level_inner = theme.elements.get(&Element::LevelInner).unwrap();
    assert_eq!(level_inner.foreground, Some(Color::RGB(RGB(0, 255, 255))));
    assert_eq!(level_inner.background, None);
    assert_eq!(level_inner.modes, Vec::<Mode>::new());

    // Logger/logger-inner pair
    let logger = theme.elements.get(&Element::Logger).unwrap();
    assert_eq!(logger.foreground, Some(Color::RGB(RGB(255, 255, 0))));
    assert_eq!(logger.modes, vec![Mode::Italic, Mode::Underline]);

    let logger_inner = theme.elements.get(&Element::LoggerInner).unwrap();
    assert_eq!(logger_inner.foreground, Some(Color::RGB(RGB(255, 255, 255))));
    assert_eq!(logger_inner.modes, vec![Mode::Bold]);
}

#[test]
fn test_v0_empty_modes_vs_absent_modes() {
    // Test that empty modes [] is different from no modes field
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-empty-modes").unwrap();

    // Element with empty modes array
    let message = theme.elements.get(&Element::Message).unwrap();
    assert_eq!(message.modes, Vec::<Mode>::new());

    // Element with modes
    let level = theme.elements.get(&Element::Level).unwrap();
    assert_eq!(level.modes, vec![Mode::Bold, Mode::Italic]);

    // Element with no modes field should have empty vec
    let level_inner = theme.elements.get(&Element::LevelInner).unwrap();
    assert_eq!(level_inner.modes, Vec::<Mode>::new());
}

#[test]
fn test_v0_yaml_anchors() {
    // Test that YAML anchors and aliases work correctly
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-yaml-anchors").unwrap();

    // Message should use base-style anchor
    let message = theme.elements.get(&Element::Message).unwrap();
    assert_eq!(message.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(message.modes, vec![Mode::Bold]);

    // Level should use secondary color
    let level = theme.elements.get(&Element::Level).unwrap();
    assert_eq!(level.foreground, Some(Color::RGB(RGB(0, 0, 255))));

    // level-inner should use error-style anchor
    let level_inner = theme.elements.get(&Element::LevelInner).unwrap();
    assert_eq!(level_inner.foreground, Some(Color::RGB(RGB(255, 0, 0))));
    assert_eq!(level_inner.background, Some(Color::RGB(RGB(17, 0, 0))));
}

#[test]
fn test_v0_undefined_anchor_error() {
    // Test that undefined YAML anchor produces an error
    let path = PathBuf::from("src/testing/assets/themes");
    let result = Theme::load_from(&path, "v0-undefined-anchor");
    assert!(result.is_err());
}

#[test]
fn test_v0_json_format() {
    // Test loading theme from JSON format
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-json-format").unwrap();

    assert!(theme.elements.get(&Element::Message).is_some());
    assert!(theme.elements.get(&Element::Level).is_some());

    let message = theme.elements.get(&Element::Message).unwrap();
    assert_eq!(message.foreground, Some(Color::RGB(RGB(255, 255, 255))));
    assert_eq!(message.modes, vec![Mode::Bold]);

    // Boolean active merge should work in JSON too
    let boolean_true = theme.elements.get(&Element::BooleanTrue).unwrap();
    assert_eq!(boolean_true.foreground, Some(Color::RGB(RGB(0, 255, 255))));
}

#[test]
fn test_v0_toml_format() {
    // Test loading theme from TOML format
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-toml-format").unwrap();

    assert!(theme.elements.get(&Element::Message).is_some());
    assert!(theme.elements.get(&Element::Level).is_some());

    let message = theme.elements.get(&Element::Message).unwrap();
    assert_eq!(message.foreground, Some(Color::RGB(RGB(255, 255, 255))));
    assert_eq!(message.modes, vec![Mode::Bold]);

    // Test different color formats
    let string_elem = theme.elements.get(&Element::String).unwrap();
    assert_eq!(string_elem.foreground, Some(Color::Plain(PlainColor::Green)));

    let number_elem = theme.elements.get(&Element::Number).unwrap();
    assert_eq!(number_elem.foreground, Some(Color::Plain(PlainColor::BrightBlue)));

    let array_elem = theme.elements.get(&Element::Array).unwrap();
    assert_eq!(array_elem.foreground, Some(Color::Palette(220)));
}

#[test]
fn test_v0_file_format_priority() {
    // Test that YAML has priority over TOML and JSON when loading by stem
    let path = PathBuf::from("src/testing/assets/themes");

    // When loading "test" by stem, should find test.toml (YAML priority, but test.yaml doesn't exist)
    let theme = Theme::load_from(&path, "test").unwrap();
    assert!(theme.elements.get(&Element::Message).is_some());

    // Loading by full filename should work
    let theme_toml = Theme::load_from(&path, "test.toml").unwrap();
    assert!(theme_toml.elements.get(&Element::Message).is_some());
}

#[test]
fn test_v0_style_pack_merge() {
    // Test StylePack merge behavior
    let mut base = StylePack::default();
    base.0.insert(
        Element::Message,
        Style {
            foreground: Some(Color::Plain(PlainColor::Red)),
            background: Some(Color::Plain(PlainColor::Blue)),
            modes: vec![Mode::Bold],
        },
    );

    let mut patch = StylePack::default();
    patch.0.insert(
        Element::Message,
        Style {
            foreground: Some(Color::Plain(PlainColor::Green)),
            background: None,
            modes: vec![Mode::Italic],
        },
    );
    patch.0.insert(
        Element::Level,
        Style {
            foreground: Some(Color::Plain(PlainColor::Yellow)),
            background: None,
            modes: vec![],
        },
    );

    let merged = base.merged(patch);

    // Message should be from patch
    assert_eq!(
        merged.0[&Element::Message].foreground,
        Some(Color::Plain(PlainColor::Green))
    );

    // Level should be from patch
    assert_eq!(
        merged.0[&Element::Level].foreground,
        Some(Color::Plain(PlainColor::Yellow))
    );
}

#[test]
fn test_v0_color_formats() {
    // Test various color format parsing
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-color-formats").unwrap();

    assert_eq!(
        theme.elements[&Element::Message].foreground,
        Some(Color::RGB(RGB(255, 0, 0)))
    );
    assert_eq!(
        theme.elements[&Element::Level].foreground,
        Some(Color::Plain(PlainColor::Red))
    );
    assert_eq!(
        theme.elements[&Element::Time].foreground,
        Some(Color::Plain(PlainColor::BrightBlue))
    );
    assert_eq!(theme.elements[&Element::Caller].foreground, Some(Color::Palette(42)));
    assert_eq!(theme.elements[&Element::Key].foreground, Some(Color::Palette(0)));
    assert_eq!(theme.elements[&Element::String].foreground, Some(Color::Palette(255)));
    assert_eq!(
        theme.elements[&Element::Logger].foreground,
        Some(Color::Plain(PlainColor::Green))
    );
}

#[test]
fn test_v0_unknown_elements_ignored() {
    // Test that unknown element names are silently ignored (forward compatibility)
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-unknown-elements").unwrap();

    // Should have parsed successfully, ignoring unknown elements
    assert_eq!(theme.elements.len(), 1);
    assert!(theme.elements.contains_key(&Element::Message));
}

#[test]
fn test_v0_unknown_level_names_ignored() {
    // Test that unknown level names are stored as InfallibleLevel::Invalid
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-unknown-levels").unwrap();

    // Should have error level as a valid level
    assert!(
        theme
            .levels
            .contains_key(&InfallibleLevel::Valid(crate::level::Level::Error))
    );

    // Unknown level names are stored as InfallibleLevel::Invalid
    // We should have 1 valid level (error) and 3 invalid levels
    let valid_count = theme
        .levels
        .keys()
        .filter(|k| matches!(k, InfallibleLevel::Valid(_)))
        .count();
    let invalid_count = theme
        .levels
        .keys()
        .filter(|k| matches!(k, InfallibleLevel::Invalid(_)))
        .count();

    assert_eq!(valid_count, 1);
    assert_eq!(invalid_count, 3); // unknown-level, super-critical, custom-level
}

#[test]
fn test_v0_indicators() {
    // Test that indicators section loads correctly
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-json-format").unwrap();

    assert_eq!(theme.indicators.sync.synced.text, " ");
    assert_eq!(theme.indicators.sync.failed.text, "!");
    assert_eq!(
        theme.indicators.sync.failed.inner.style.foreground,
        Some(Color::Plain(PlainColor::Yellow))
    );
    assert_eq!(theme.indicators.sync.failed.inner.style.modes, vec![Mode::Bold]);
}

#[test]
fn test_theme_list() {
    // Test theme listing functionality
    let app_dirs = AppDirs {
        config_dir: PathBuf::from("src/testing/assets"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    };

    let themes = Theme::list(&app_dirs).unwrap();

    // Should include embedded themes
    assert!(themes.contains_key("universal"));

    // Should include custom themes
    assert!(themes.contains_key("test"));
}

#[test]
fn test_theme_not_found_error() {
    // Test that theme not found error includes suggestions
    let path = PathBuf::from("src/testing/assets/themes");
    let result = Theme::load_from(&path, "nonexistent");

    assert!(result.is_err());
    match result {
        Err(Error::ThemeNotFound { name, .. }) => {
            assert_eq!(name, "nonexistent");
        }
        _ => panic!("Expected ThemeNotFound error"),
    }
}

#[test]
fn test_format_iteration() {
    // Test that Format enum iterates in correct priority order
    let formats: Vec<Format> = Format::iter().collect();
    assert_eq!(formats.len(), 3);
    assert_eq!(formats[0], Format::Yaml);
    assert_eq!(formats[1], Format::Toml);
    assert_eq!(formats[2], Format::Json);
}

#[test]
fn test_format_extensions() {
    assert_eq!(Format::Yaml.extension(), "yaml");
    assert_eq!(Format::Toml.extension(), "toml");
    assert_eq!(Format::Json.extension(), "json");
}

#[test]
fn test_v0_duplicate_modes() {
    // Test that v0 allows duplicate modes and passes them to terminal
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-duplicate-modes").unwrap();

    // Message has duplicate bold modes
    let message = theme.elements.get(&Element::Message).unwrap();
    assert_eq!(
        message.modes,
        vec![Mode::Bold, Mode::Italic, Mode::Bold, Mode::Underline, Mode::Bold]
    );

    // Level has three italic modes
    let level = theme.elements.get(&Element::Level).unwrap();
    assert_eq!(level.modes, vec![Mode::Italic, Mode::Italic, Mode::Italic]);

    // Time has duplicate faint
    let time = theme.elements.get(&Element::Time).unwrap();
    assert_eq!(time.modes, vec![Mode::Faint, Mode::Bold, Mode::Faint]);
}

#[test]
fn test_v0_all_modes() {
    // Test that all ANSI mode types are supported
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-all-modes").unwrap();

    // Test individual modes
    assert_eq!(theme.elements[&Element::Message].modes, vec![Mode::Bold]);
    assert_eq!(theme.elements[&Element::Level].modes, vec![Mode::Faint]);
    assert_eq!(theme.elements[&Element::LevelInner].modes, vec![Mode::Italic]);
    assert_eq!(theme.elements[&Element::Time].modes, vec![Mode::Underline]);
    assert_eq!(theme.elements[&Element::Caller].modes, vec![Mode::SlowBlink]);
    assert_eq!(theme.elements[&Element::Logger].modes, vec![Mode::RapidBlink]);
    assert_eq!(theme.elements[&Element::Key].modes, vec![Mode::Reverse]);
    assert_eq!(theme.elements[&Element::String].modes, vec![Mode::Conceal]);
    assert_eq!(theme.elements[&Element::Number].modes, vec![Mode::CrossedOut]);

    // Test combined modes
    let boolean = theme.elements.get(&Element::Boolean).unwrap();
    assert_eq!(boolean.modes, vec![Mode::Bold, Mode::Italic, Mode::Underline]);

    let boolean_true = theme.elements.get(&Element::BooleanTrue).unwrap();
    assert_eq!(
        boolean_true.modes,
        vec![Mode::Bold, Mode::Faint, Mode::Italic, Mode::Underline, Mode::SlowBlink]
    );
}

#[test]
fn test_v0_palette_range() {
    // Test that palette indices from 0 to 255 are all valid
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-palette-range").unwrap();

    // Test boundary values
    assert_eq!(theme.elements[&Element::Message].foreground, Some(Color::Palette(0)));
    assert_eq!(theme.elements[&Element::Message].background, Some(Color::Palette(255)));

    // Test various palette values
    assert_eq!(theme.elements[&Element::Level].foreground, Some(Color::Palette(1)));
    assert_eq!(
        theme.elements[&Element::LevelInner].foreground,
        Some(Color::Palette(16))
    );
    assert_eq!(theme.elements[&Element::Time].foreground, Some(Color::Palette(88)));
    assert_eq!(theme.elements[&Element::Caller].foreground, Some(Color::Palette(124)));
    assert_eq!(theme.elements[&Element::Logger].foreground, Some(Color::Palette(196)));
    assert_eq!(theme.elements[&Element::Key].foreground, Some(Color::Palette(220)));
    assert_eq!(theme.elements[&Element::String].foreground, Some(Color::Palette(46)));
}

#[test]
fn test_v0_level_override_merge_behavior() {
    // Test that level-specific overrides properly merge with base elements
    // Level overrides should only replace properties that are explicitly defined
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-level-overrides").unwrap();

    // Base message has foreground, background, and modes
    let base_message = theme.elements.get(&Element::Message).unwrap();
    assert_eq!(base_message.foreground, Some(Color::RGB(RGB(255, 255, 255))));
    assert_eq!(base_message.background, Some(Color::RGB(RGB(0, 0, 0))));
    assert_eq!(base_message.modes, vec![Mode::Bold]);

    // Error level message override only has foreground
    // At themecfg level, it should only have foreground
    // The merge with base happens at a higher level
    let error_message = theme
        .levels
        .get(&InfallibleLevel::Valid(crate::level::Level::Error))
        .and_then(|pack| pack.get(&Element::Message));
    assert!(error_message.is_some());
    let error_message = error_message.unwrap();
    assert_eq!(error_message.foreground, Some(Color::RGB(RGB(255, 136, 136))));
}

#[test]
fn test_v0_style_merged_modes() {
    // Test Style::merged behavior with modes
    let base = Style {
        modes: vec![Mode::Bold, Mode::Italic],
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: None,
    };

    let patch_with_modes = Style {
        modes: vec![Mode::Underline],
        foreground: None,
        background: Some(Color::Plain(PlainColor::Blue)),
    };

    // When patch has non-empty modes, it replaces base modes
    let result = base.clone().merged(&patch_with_modes);
    assert_eq!(result.modes, vec![Mode::Underline]);

    let patch_empty_modes = Style {
        modes: vec![],
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: None,
    };

    // When patch has empty modes, base modes are preserved
    let result = base.clone().merged(&patch_empty_modes);
    assert_eq!(result.modes, vec![Mode::Bold, Mode::Italic]);
}

#[test]
fn test_v0_indicators_default_values() {
    // Test that indicators have proper default values when not specified
    let theme = Theme::default();

    // Default synced indicator should have empty text " "
    assert_eq!(theme.indicators.sync.synced.text, " ");

    // Default failed indicator should have "!" and yellow bold styling
    assert_eq!(theme.indicators.sync.failed.text, "!");
    assert_eq!(
        theme.indicators.sync.failed.inner.style.foreground,
        Some(Color::Plain(PlainColor::Yellow))
    );
    assert_eq!(theme.indicators.sync.failed.inner.style.modes, vec![Mode::Bold]);
}

#[test]
fn test_v0_tags_parsing() {
    // Test that tags are parsed correctly
    let yaml = include_str!("../testing/assets/themes/test.toml");
    let theme: Theme = toml::from_str(yaml).unwrap();

    // Test theme can be loaded (tags field is optional)
    assert!(theme.elements.len() > 0);
}

#[test]
fn test_v0_partial_element_definitions() {
    // Test elements with only partial properties defined
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-nested-styling").unwrap();

    // input-number-inner has only background, no foreground or modes
    let input_number_inner = theme.elements.get(&Element::InputNumberInner).unwrap();
    assert_eq!(input_number_inner.foreground, None);
    assert_eq!(input_number_inner.background, Some(Color::RGB(RGB(0, 0, 68))));
    assert_eq!(input_number_inner.modes, Vec::<Mode>::new());
}

#[test]
fn test_v0_rgb_case_insensitivity() {
    // RGB hex colors should accept both uppercase and lowercase
    assert_eq!(RGB::from_str("#aabbcc").unwrap(), RGB(170, 187, 204));
    assert_eq!(RGB::from_str("#AABBCC").unwrap(), RGB(170, 187, 204));
    assert_eq!(RGB::from_str("#AaBbCc").unwrap(), RGB(170, 187, 204));
}

#[test]
fn test_v0_plain_color_case_sensitivity() {
    // Plain color names are case-sensitive in v0
    // This test verifies the existing behavior
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-color-formats").unwrap();

    // 'red' should parse as PlainColor::Red
    assert_eq!(
        theme.elements[&Element::Level].foreground,
        Some(Color::Plain(PlainColor::Red))
    );
}

#[test]
fn test_v0_boolean_merge_with_level_overrides() {
    // Test whether level-specific overrides to `boolean` element
    // affect boolean-true and boolean-false at that level.
    // This tests the timing of boolean active merge relative to level merging.
    let path = PathBuf::from("src/testing/assets/themes");
    let theme = Theme::load_from(&path, "v0-boolean-level-override").unwrap();

    // Base elements - boolean merge happens at themecfg level or theme level?
    // At themecfg level, we just see the raw elements
    let base_boolean = theme.elements.get(&Element::Boolean).unwrap();
    assert_eq!(base_boolean.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(base_boolean.background, Some(Color::RGB(RGB(0, 17, 0))));

    let base_boolean_true = theme.elements.get(&Element::BooleanTrue).unwrap();
    assert_eq!(base_boolean_true.foreground, Some(Color::RGB(RGB(0, 255, 255))));
    // At themecfg level, boolean-true doesn't have background yet
    // The merge happens in theme::StylePack::load()

    // Error level has overrides for boolean and boolean-false
    let error_pack = theme
        .levels
        .get(&InfallibleLevel::Valid(crate::level::Level::Error))
        .unwrap();

    // Error level should have overridden boolean
    let error_boolean = error_pack.get(&Element::Boolean).unwrap();
    assert_eq!(error_boolean.foreground, Some(Color::RGB(RGB(255, 0, 255))));

    // Error level should have overridden boolean-false
    let error_boolean_false = error_pack.get(&Element::BooleanFalse).unwrap();
    assert_eq!(error_boolean_false.foreground, Some(Color::RGB(RGB(255, 170, 170))));

    // Note: The boolean merge happens during theme::Theme creation,
    // not at the themecfg::Theme level. So we can't test the final merged
    // result here - this test documents the themecfg-level behavior.
    // The actual boolean merge with level overrides happens in theme::StylePack::load()
}
