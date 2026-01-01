use std::{path::PathBuf, str::FromStr};

use strum::IntoEnumIterator;

use crate::{appdirs::AppDirs, level::Level};

use super::super::{
    Color, Element, Error, Format, Mode, ModeSetDiff, PlainColor, RGB, Role, Style, Tag, Theme, Version,
    tests::{dirs, load_raw_theme, raw_theme, theme},
};

#[test]
fn test_load() {
    let dirs = dirs();
    assert_ne!(Theme::load(&dirs, "test").unwrap().elements.len(), 0);
    assert_ne!(Theme::load(&dirs, "universal").unwrap().elements.len(), 0);
    assert!(Theme::load(&dirs, "non-existent").is_err());
    assert!(Theme::load(&dirs, "invalid").is_err());
    assert!(Theme::load(&dirs, "invalid-type").is_err());
}

#[test]
fn test_v0_input_element_inheritance() {
    let theme = theme("v0-color-formats");

    let expected = Style {
        foreground: Some(Color::Plain(PlainColor::BrightYellow)),
        ..Default::default()
    };

    let input = theme.elements.get(&Element::Input);
    assert!(
        input.is_some(),
        "Input element should be present in v0 theme after merge with @base"
    );
    assert_eq!(
        input,
        Some(&expected),
        "Input element should have bright-yellow foreground"
    );

    assert_eq!(
        theme.elements.get(&Element::InputNumber),
        Some(&expected),
        "InputNumber should be inherited when v0 theme defines Input"
    );
    assert_eq!(
        theme.elements.get(&Element::InputName),
        Some(&expected),
        "InputName should be inherited when v0 theme defines Input"
    );
}

#[test]
fn test_load_from() {
    let app_dirs = dirs();
    assert_ne!(Theme::load(&app_dirs, "universal").unwrap().elements.len(), 0);

    assert_ne!(Theme::load(&app_dirs, "test").unwrap().elements.len(), 0);
    assert_ne!(Theme::load(&app_dirs, "test.toml").unwrap().elements.len(), 0);
    assert_ne!(
        Theme::load(&app_dirs, "./src/testing/assets/themes/test.toml")
            .unwrap()
            .elements
            .len(),
        0
    );
    assert!(Theme::load(&app_dirs, "non-existent").is_err());
    assert!(Theme::load(&app_dirs, "invalid").is_err());
    assert!(Theme::load(&app_dirs, "invalid-type").is_err());
}

#[test]
fn test_embedded() {
    assert_ne!(Theme::embedded("universal").unwrap().elements.len(), 0);
    assert!(Theme::embedded("non-existent").is_err());
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
fn test_v0_boolean_active_merge() {
    let theme = theme("v0-boolean-merge");

    assert!(theme.elements.get(&Element::Boolean).is_some());
    let boolean = &theme.elements[&Element::Boolean];
    assert_eq!(boolean.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(boolean.background, Some(Color::RGB(RGB(0, 0, 0))));
    assert_eq!(boolean.modes, Mode::Bold.into());

    let boolean_true = &theme.elements[&Element::BooleanTrue];
    assert_eq!(boolean_true.foreground, Some(Color::RGB(RGB(0, 255, 255))));

    let boolean_false = &theme.elements[&Element::BooleanFalse];
    assert_eq!(boolean_false.foreground, Some(Color::RGB(RGB(255, 0, 0))));
}

#[test]
fn test_v0_modes_replacement() {
    let theme = theme("v0-modes-replace");

    let level = &theme.elements[&Element::Level];
    assert_eq!(level.modes, (Mode::Bold | Mode::Underline).into());

    let level_inner = &theme.elements[&Element::LevelInner];
    assert_eq!(level_inner.modes, Mode::Italic.into());
}

#[test]
fn test_v0_level_specific_overrides() {
    let theme = theme("v0-level-overrides");

    let base_level = &theme.elements[&Element::Level];
    assert_eq!(base_level.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(base_level.modes, Mode::Italic.into());

    let debug_level = theme
        .levels
        .get(&Level::Debug)
        .and_then(|pack| pack.get(&Element::Level));
    assert!(debug_level.is_some());
    let debug_level = debug_level.unwrap();
    assert_eq!(debug_level.foreground, Some(Color::RGB(RGB(255, 0, 255))));
    assert_eq!(debug_level.modes, (Mode::Bold | Mode::Underline).into());

    let error_level = theme
        .levels
        .get(&Level::Error)
        .and_then(|pack| pack.get(&Element::Level));
    assert!(error_level.is_some());
    let error_level = error_level.unwrap();
    assert_eq!(error_level.foreground, Some(Color::RGB(RGB(255, 0, 0))));
    assert_eq!(error_level.background, Some(Color::RGB(RGB(68, 0, 0))));
    assert_eq!(error_level.modes, (Mode::Reverse | Mode::Bold).into());
}

#[test]
fn test_v0_nested_styling_elements() {
    let theme = theme("v0-nested-styling");

    let level = &theme.elements[&Element::Level];
    assert_eq!(level.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(level.background, Some(Color::RGB(RGB(0, 17, 0))));
    assert_eq!(level.modes, Mode::Bold.into());

    let level_inner = &theme.elements[&Element::LevelInner];
    assert_eq!(level_inner.foreground, Some(Color::RGB(RGB(0, 255, 255))));
    assert_eq!(level_inner.background, None);
    assert_eq!(level_inner.modes, ModeSetDiff::new());

    let logger = &theme.elements[&Element::Logger];
    assert_eq!(logger.foreground, Some(Color::RGB(RGB(255, 255, 0))));
    assert_eq!(logger.modes, (Mode::Italic | Mode::Underline).into());

    let logger_inner = &theme.elements[&Element::LoggerInner];
    assert_eq!(logger_inner.foreground, Some(Color::RGB(RGB(255, 255, 255))));
    assert_eq!(logger_inner.modes, Mode::Bold.into());
}

#[test]
fn test_v0_empty_modes_vs_absent_modes() {
    let theme = theme("v0-empty-modes");

    let message = &theme.elements[&Element::Message];
    assert_eq!(message.modes, ModeSetDiff::new());

    let level = &theme.elements[&Element::Level];
    assert_eq!(level.modes, (Mode::Bold | Mode::Italic).into());

    let level_inner = &theme.elements[&Element::LevelInner];
    assert_eq!(level_inner.modes, ModeSetDiff::new());
}

#[test]
fn test_v0_yaml_anchors() {
    let theme = theme("v0-yaml-anchors");

    let message = &theme.elements[&Element::Message];
    assert_eq!(message.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(message.modes, Mode::Bold.into());

    let level = &theme.elements[&Element::Level];
    assert_eq!(level.foreground, Some(Color::RGB(RGB(0, 0, 255))));

    let level_inner = &theme.elements[&Element::LevelInner];
    assert_eq!(level_inner.foreground, Some(Color::RGB(RGB(255, 0, 0))));
    assert_eq!(level_inner.background, Some(Color::RGB(RGB(17, 0, 0))));
}

#[test]
fn test_v0_undefined_anchor_error() {
    let result = Theme::load(&dirs(), "v0-undefined-anchor");
    assert!(result.is_err());
}

#[test]
fn test_v0_json_format() {
    let theme = theme("v0-json-format");

    assert!(theme.elements.get(&Element::Message).is_some());
    assert!(theme.elements.get(&Element::Level).is_some());

    let message = &theme.elements[&Element::Message];
    assert_eq!(message.foreground, Some(Color::RGB(RGB(255, 255, 255))));
    assert_eq!(message.modes, Mode::Bold.into());

    let boolean_true = &theme.elements[&Element::BooleanTrue];
    assert_eq!(boolean_true.foreground, Some(Color::RGB(RGB(0, 255, 255))));
}

#[test]
fn test_v0_toml_format() {
    let theme = theme("v0-toml-format");

    assert!(theme.elements.get(&Element::Message).is_some());
    assert!(theme.elements.get(&Element::Level).is_some());

    let message = &theme.elements[&Element::Message];
    assert_eq!(message.foreground, Some(Color::RGB(RGB(255, 255, 255))));
    assert_eq!(message.modes, Mode::Bold.into());

    let string_elem = &theme.elements[&Element::String];
    assert_eq!(string_elem.foreground, Some(Color::Plain(PlainColor::Green)));

    let number_elem = &theme.elements[&Element::Number];
    assert_eq!(number_elem.foreground, Some(Color::Plain(PlainColor::BrightBlue)));

    let array_elem = &theme.elements[&Element::Array];
    assert_eq!(array_elem.foreground, Some(Color::Palette(220)));
}

#[test]
fn test_v0_file_format_priority() {
    let dirs = dirs();

    let theme = Theme::load(&dirs, "test").unwrap();
    assert!(theme.elements.get(&Element::Message).is_some());

    let theme = Theme::load(&dirs, "test.toml").unwrap();
    assert!(theme.elements.get(&Element::Message).is_some());
}

#[test]
fn test_format_iteration() {
    let formats: Vec<Format> = Format::iter().collect();
    assert_eq!(formats.len(), 3);
    assert_eq!(formats[0], Format::Yaml);
    assert_eq!(formats[1], Format::Toml);
    assert_eq!(formats[2], Format::Json);
}

#[test]
fn test_format_extensions() {
    assert_eq!(Format::Yaml.extensions(), &["yaml", "yml"]);
    assert_eq!(Format::Toml.extensions(), &["toml"]);
    assert_eq!(Format::Json.extensions(), &["json"]);
}

#[test]
fn test_v0_indicators() {
    let theme = theme("v0-json-format");

    assert_eq!(theme.indicators.sync.synced.text, " ");
    assert_eq!(theme.indicators.sync.failed.text, "!");
    assert_eq!(
        theme.indicators.sync.failed.inner.style.foreground,
        Some(Color::Plain(PlainColor::Yellow))
    );
    assert_eq!(theme.indicators.sync.failed.inner.style.modes, Mode::Bold.into());
}

#[test]
fn test_theme_list() {
    let themes = Theme::list(&dirs()).unwrap();

    assert!(themes.contains_key("universal"));

    assert!(themes.contains_key("test"));
}

#[test]
fn test_theme_not_found_error() {
    let result = Theme::load(&dirs(), "nonexistent");

    assert!(result.is_err());
    match result {
        Err(Error::ThemeNotFound { name, .. }) => {
            assert_eq!(name.as_ref(), "nonexistent");
        }
        _ => panic!("Expected ThemeNotFound error"),
    }
}

#[test]
fn test_v0_duplicate_modes() {
    let theme = theme("v0-duplicate-modes");

    let message = &theme.elements[&Element::Message];
    assert_eq!(message.modes, (Mode::Bold | Mode::Italic | Mode::Underline).into());

    let level = &theme.elements[&Element::Level];
    assert_eq!(level.modes, (Mode::Italic).into());

    let time = &theme.elements[&Element::Time];
    assert_eq!(time.modes, (Mode::Faint | Mode::Bold).into());
}

#[test]
fn test_v0_all_modes() {
    let theme = theme("v0-all-modes");

    assert_eq!(theme.elements[&Element::Message].modes, Mode::Bold.into());
    assert_eq!(theme.elements[&Element::Level].modes, Mode::Faint.into());
    assert_eq!(theme.elements[&Element::LevelInner].modes, Mode::Italic.into());
    assert_eq!(theme.elements[&Element::Time].modes, Mode::Underline.into());
    assert_eq!(theme.elements[&Element::Caller].modes, Mode::SlowBlink.into());
    assert_eq!(theme.elements[&Element::Logger].modes, Mode::RapidBlink.into());
    assert_eq!(theme.elements[&Element::Key].modes, Mode::Reverse.into());
    assert_eq!(theme.elements[&Element::String].modes, Mode::Conceal.into());
    assert_eq!(theme.elements[&Element::Number].modes, Mode::CrossedOut.into());

    let boolean = &theme.elements[&Element::Boolean];
    assert_eq!(boolean.modes, (Mode::Bold | Mode::Italic | Mode::Underline).into());

    let boolean_true = &theme.elements[&Element::BooleanTrue];
    assert_eq!(
        boolean_true.modes,
        (Mode::Bold | Mode::Faint | Mode::Italic | Mode::Underline | Mode::SlowBlink).into(),
    );
}

#[test]
fn test_v0_palette_range() {
    let theme = theme("v0-palette-range");

    assert_eq!(theme.elements[&Element::Message].foreground, Some(Color::Palette(0)));
    assert_eq!(theme.elements[&Element::Message].background, Some(Color::Palette(255)));

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
    let theme = theme("v0-level-overrides");

    let base_message = &theme.elements[&Element::Message];
    assert_eq!(base_message.foreground, Some(Color::RGB(RGB(255, 255, 255))));
    assert_eq!(base_message.background, Some(Color::RGB(RGB(0, 0, 0))));
    assert_eq!(base_message.modes, Mode::Bold.into());

    let error_message = theme
        .levels
        .get(&Level::Error)
        .and_then(|pack| pack.get(&Element::Message));
    assert!(error_message.is_some());
    let error_message = error_message.unwrap();
    assert_eq!(error_message.foreground, Some(Color::RGB(RGB(255, 136, 136))));
}

#[test]
fn test_unknown_elements_toml() {
    let result = load_raw_theme("test-unknown-elements.toml");

    match result {
        Ok(theme) => {
            assert_eq!(theme.elements.len(), 2, "Should only load 2 known elements from file");
            assert!(theme.elements.contains_key(&Element::Message));
            assert!(theme.elements.contains_key(&Element::Level));

            let theme = theme.resolve().unwrap();
            assert_eq!(
                theme.elements.len(),
                3,
                "Should have 3 elements after resolution (Level + LevelInner + Message)"
            );
            assert!(theme.elements.contains_key(&Element::Message));
            assert!(theme.elements.contains_key(&Element::Level));
            assert!(theme.elements.contains_key(&Element::LevelInner));
        }
        Err(e) => {
            panic!("TOML with unknown elements failed: {:?}", e);
        }
    }
}

#[test]
fn test_unknown_elements_json() {
    let result = load_raw_theme("test-unknown-elements.json");

    match result {
        Ok(theme) => {
            assert_eq!(theme.elements.len(), 2, "Should only load 2 known elements from file");
            assert!(theme.elements.contains_key(&Element::Message));
            assert!(theme.elements.contains_key(&Element::Level));

            let theme = theme.resolve().unwrap();
            assert_eq!(
                theme.elements.len(),
                3,
                "Should have 3 elements after resolution (Level + LevelInner + Message)"
            );
            assert!(theme.elements.contains_key(&Element::Message));
            assert!(theme.elements.contains_key(&Element::Level));
            assert!(theme.elements.contains_key(&Element::LevelInner));
        }
        Err(e) => {
            panic!("JSON with unknown elements failed: {:?}", e);
        }
    }
}

#[test]
fn test_unknown_elements_yaml() {
    let result = load_raw_theme("test-unknown-elements.yaml");

    match result {
        Ok(theme) => {
            assert_eq!(theme.elements.len(), 2, "Should only load 2 known elements from file");
            assert!(theme.elements.contains_key(&Element::Message));
            assert!(theme.elements.contains_key(&Element::Level));

            let theme = theme.resolve().unwrap();
            assert_eq!(
                theme.elements.len(),
                3,
                "Should have 3 elements after resolution (Level + LevelInner + Message)"
            );
            assert!(theme.elements.contains_key(&Element::Message));
            assert!(theme.elements.contains_key(&Element::Level));
            assert!(theme.elements.contains_key(&Element::LevelInner));
        }
        Err(e) => {
            panic!("YAML with unknown elements failed: {:?}", e);
        }
    }
}

#[test]
fn test_v0_indicators_default_values() {
    let theme = theme("@base");

    assert_eq!(theme.indicators.sync.synced.text, " ");

    assert_eq!(theme.indicators.sync.failed.text, "!");
}

#[test]
fn test_v1_element_replacement_preserves_per_level_modes() {
    let app_dirs = dirs();
    let theme = Theme::load_raw(&app_dirs, "v1-element-modes-per-level").unwrap();

    let level_inner = theme.elements.get(&Element::LevelInner);
    assert!(level_inner.is_some(), "level-inner element should exist");
    assert!(
        level_inner.unwrap().modes.adds.contains(Mode::Bold),
        "level-inner element should have bold mode"
    );

    let info_level = theme.levels.get(&Level::Info);
    assert!(info_level.is_some(), "info level should exist");
    let info_level_inner = info_level.unwrap().get(&Element::LevelInner);
    assert!(info_level_inner.is_some(), "info level-inner should exist");

    assert!(
        !info_level_inner.unwrap().base.is_empty(),
        "info level-inner should have a style base"
    );
}

#[test]
fn test_v0_partial_element_definitions() {
    let theme = theme("v0-nested-styling");

    let inner = &theme.elements[&Element::InputNumberInner];
    assert_eq!(inner.foreground, None);
    assert_eq!(inner.background, Some(Color::RGB(RGB(0, 0, 68))));
    assert_eq!(inner.modes, ModeSetDiff::new());
}

#[test]
fn test_v0_boolean_merge_with_level_overrides() {
    let theme = theme("v0-boolean-level-override");

    let base_boolean = &theme.elements[&Element::Boolean];
    assert_eq!(base_boolean.foreground, Some(Color::RGB(RGB(0, 255, 0))));
    assert_eq!(base_boolean.background, Some(Color::RGB(RGB(0, 17, 0))));

    let base_boolean_true = &theme.elements[&Element::BooleanTrue];
    assert_eq!(base_boolean_true.foreground, Some(Color::RGB(RGB(0, 255, 255))));

    let error_pack = &theme.levels[&Level::Error];

    let error_boolean = &error_pack[&Element::Boolean];
    assert_eq!(error_boolean.foreground, Some(Color::RGB(RGB(255, 0, 255))));

    let error_boolean_false = &error_pack[&Element::BooleanFalse];
    assert_eq!(error_boolean_false.foreground, Some(Color::RGB(RGB(255, 170, 170))));
}

#[test]
fn test_custom_default_theme_with_extension() {
    let theme = theme("@base.yaml");

    assert_eq!(
        theme.version,
        Version::V0_0,
        "Custom @base.yaml is v0, merged result uses custom theme's version"
    );

    let message_style = theme.elements.get(&Element::Message);
    assert!(
        message_style.is_some(),
        "Message element should be present (from custom or @base)"
    );

    assert_eq!(
        message_style.unwrap().foreground,
        Some(Color::Plain(PlainColor::Red)),
        "Custom @base.yaml message definition should override embedded @base"
    );

    assert!(
        theme.elements.get(&Element::Input).is_some(),
        "Should have 'input' element from embedded @base (not in custom file)"
    );
    assert!(
        theme.elements.get(&Element::Time).is_some(),
        "Should have 'time' element from embedded @base (not in custom file)"
    );

    assert!(
        theme.elements.len() > 1,
        "Should have multiple elements from @base merge, not just 'message' from custom file. Got {} elements",
        theme.elements.len()
    );
}

#[test]
fn test_v0_rejects_mode_prefix() {
    let result = Theme::load(&dirs(), "v0-invalid-mode-prefix");

    assert!(result.is_err(), "V0 theme with - mode prefix should fail to load");

    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("mode prefix") || error_msg.contains("v0") || error_msg.contains("v1.0"),
            "Error should mention mode prefix issue, got: {}",
            error_msg
        );
    }
}

#[test]
fn test_filesystem_error_handling() {
    let result = Theme::load(&dirs(), "definitely-does-not-exist-12345");
    assert!(result.is_err(), "Should fail when theme file doesn't exist");

    match result {
        Err(Error::ThemeNotFound { name, .. }) => {
            assert_eq!(name.as_ref(), "definitely-does-not-exist-12345");
        }
        _ => panic!("Expected ThemeNotFound error for non-existent file"),
    }

    let invalid_path = PathBuf::from("/nonexistent/directory/that/does/not/exist");
    let result = Theme::load_from(&invalid_path, "any-theme");
    assert!(result.is_err(), "Should fail when directory doesn't exist");
}

#[test]
fn test_mode_names_case_sensitive() {
    let result = Theme::load(&dirs(), "v0-invalid-mode-case");

    assert!(
        result.is_err(),
        "Theme with invalid mode case 'Bold' should fail to load"
    );

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("Bold") || error_msg.contains("mode") || error_msg.contains("unknown"),
            "Error should mention invalid mode, got: {}",
            error_msg
        );
    }
}

#[test]
fn test_tag_validation() {
    let result = Theme::load(&dirs(), "v0-invalid-tag");

    assert!(result.is_err(), "Theme with invalid tag value should fail to load");

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("tag") || error_msg.contains("invalid"),
            "Error should mention invalid tag, got: {}",
            error_msg
        );
    }
}

#[test]
fn test_multiple_conflicting_tags_allowed() {
    let theme = theme("v0-multiple-tags");

    assert_eq!(theme.tags.len(), 4, "Should have 4 tags");

    assert!(theme.tags.contains(Tag::Dark), "Should have 'dark' tag");
    assert!(theme.tags.contains(Tag::Light), "Should have 'light' tag");
    assert!(theme.tags.contains(Tag::Palette256), "Should have '256color' tag");
    assert!(theme.tags.contains(Tag::TrueColor), "Should have 'truecolor' tag");
}

#[test]
fn test_custom_default_theme_without_extension() {
    let theme = theme("@base");

    assert_eq!(
        theme.version,
        Version::V0_0,
        "Custom @base is v0, merged result uses custom theme's version"
    );

    let message_style = theme.elements.get(&Element::Message);
    assert!(message_style.is_some(), "Message element should be present after merge");

    assert_eq!(
        message_style.unwrap().foreground,
        Some(Color::Plain(PlainColor::Red)),
        "Custom @base.yaml message definition should override embedded @base"
    );

    assert!(
        theme.elements.get(&Element::Input).is_some(),
        "Should have 'input' element from embedded @base (not in custom file)"
    );
    assert!(
        theme.elements.get(&Element::Time).is_some(),
        "Should have 'time' element from embedded @base (not in custom file)"
    );

    assert!(
        theme.elements.len() > 1,
        "Should have multiple elements from @base merge, not just 'message' from custom file. Got {} elements",
        theme.elements.len()
    );
}

#[test]
fn test_load_by_full_filename_explicit() {
    let toml_theme = theme("test-fullname.toml");

    assert_eq!(
        toml_theme.elements[&Element::Key].foreground,
        Some(Color::Plain(PlainColor::Magenta)),
        "Loading test-fullname.toml should load TOML file with magenta key"
    );
    assert_eq!(
        toml_theme.elements[&Element::Number].foreground,
        Some(Color::Plain(PlainColor::Yellow)),
        "TOML file should have yellow number"
    );
    assert_eq!(
        toml_theme.elements[&Element::Message].foreground,
        Some(Color::Plain(PlainColor::Blue)),
        "TOML file should have blue message"
    );

    let yaml_theme = theme("test-fullname.yaml");

    assert_eq!(
        yaml_theme.elements[&Element::Key].foreground,
        Some(Color::Plain(PlainColor::Cyan)),
        "Loading test-fullname.yaml should load YAML file with cyan key"
    );
    assert_eq!(
        yaml_theme.elements[&Element::Number].foreground,
        Some(Color::Plain(PlainColor::Green)),
        "YAML file should have green number"
    );
    assert_eq!(
        yaml_theme.elements[&Element::Message].foreground,
        Some(Color::Plain(PlainColor::White)),
        "YAML file should have white message"
    );
}

#[test]
fn test_silent_on_success() {
    let result = Theme::load(&dirs(), "test-fullname.yaml");

    assert!(result.is_ok(), "Theme load should succeed silently");

    let result = Theme::load(&dirs(), "test");
    assert!(result.is_ok(), "Theme load via AppDirs should succeed silently");
}

#[test]
fn test_theme_stem_deduplication() {
    let themes = Theme::list(&dirs()).unwrap();

    let dedup_count = themes.keys().filter(|k| k.as_ref() == "dedup-test").count();

    assert_eq!(
        dedup_count, 1,
        "Theme stem 'dedup-test' should appear exactly once in listing, even though both .yaml and .toml exist"
    );

    assert!(
        themes.contains_key("dedup-test"),
        "dedup-test should be present in theme listing"
    );
}

#[test]
fn test_custom_theme_priority_over_stock() {
    let theme = theme("universal");

    assert_eq!(
        theme.elements[&Element::Key].foreground,
        Some(Color::RGB(RGB(255, 0, 255))),
        "Custom universal theme should override stock: key should be bright magenta #FF00FF"
    );
    assert_eq!(
        theme.elements[&Element::Message].foreground,
        Some(Color::RGB(RGB(0, 255, 255))),
        "Custom universal theme should override stock: message should be bright cyan #00FFFF"
    );
    assert_eq!(
        theme.elements[&Element::Time].foreground,
        Some(Color::RGB(RGB(255, 255, 0))),
        "Custom universal theme should override stock: time should be bright yellow #FFFF00"
    );
    assert_eq!(
        theme.elements[&Element::Level].foreground,
        Some(Color::RGB(RGB(255, 0, 0))),
        "Custom universal theme should override stock: level should be bright red #FF0000"
    );
}

#[test]
fn test_platform_specific_paths() {
    let result = Theme::load(&dirs(), "test");
    assert!(
        result.is_ok(),
        "Theme should load from custom config_dir path via AppDirs"
    );

    let different_app_dirs = AppDirs {
        config_dir: PathBuf::from("etc/defaults"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    };

    let result = Theme::load(&different_app_dirs, "test");
    assert!(
        result.is_err(),
        "Theme 'test' should not be found in different config_dir (etc/defaults)"
    );

    let theme = Theme::load(&dirs(), "test").unwrap();
    assert!(
        !theme.elements.is_empty(),
        "Theme loaded from custom AppDirs should have elements"
    );
}

#[test]
fn test_theme_name_suggestions() {
    let result = Theme::load(&dirs(), "universl");
    assert!(result.is_err(), "Loading non-existent theme should fail");

    match result.unwrap_err() {
        Error::ThemeNotFound { name, suggestions } => {
            assert_eq!(name.as_ref(), "universl");
            assert!(
                !suggestions.is_empty(),
                "Should provide suggestions for typo 'universl' (likely 'universal')"
            );
        }
        other => panic!("Expected ThemeNotFound error, got: {:?}", other),
    }

    let result = Theme::load(&dirs(), "tst");
    assert!(result.is_err(), "Loading non-existent theme should fail");

    match result.unwrap_err() {
        Error::ThemeNotFound { name, suggestions } => {
            assert_eq!(name.as_ref(), "tst");
            assert!(
                !suggestions.is_empty(),
                "Should provide suggestions for typo 'tst' (likely 'test')"
            );
        }
        other => panic!("Expected ThemeNotFound error, got: {:?}", other),
    }
}

#[test]
fn test_v1_level_overrides_with_styles() {
    let theme = raw_theme("v1-level-with-styles");

    assert_eq!(theme.version, Version::V1_0);

    assert!(!theme.styles.is_empty(), "V1 theme should have style definitions");

    assert_eq!(
        theme.elements[&Element::Message].foreground,
        Some(Color::RGB(RGB(255, 255, 255))),
        "Base message should be white"
    );

    let error_level = Level::Error;
    assert!(
        theme.levels.contains_key(&error_level),
        "Theme should have error level overrides"
    );

    let error_message = theme
        .levels
        .get(&error_level)
        .and_then(|pack| pack.get(&Element::Message));
    assert!(error_message.is_some(), "Error level should override message element");

    let error_msg_style = error_message.unwrap();
    assert!(
        !error_msg_style.base.is_empty(),
        "V1 level override should reference styles via base"
    );
}

#[test]
fn test_v1_level_override_foreground() {
    let theme = theme("v1-level-override-foreground");

    assert_eq!(
        theme.elements[&Element::Level],
        Style {
            foreground: Some(Color::Palette(139)),
            ..Default::default()
        }
    );

    assert_eq!(
        theme.elements[&Element::LevelInner],
        Style {
            foreground: Some(Color::Palette(139)),
            modes: ModeSetDiff::new() - Mode::Faint,
            ..Default::default()
        }
    );

    assert_eq!(
        theme.levels[&Level::Warning][&Element::Level],
        Style {
            foreground: Some(Color::Palette(139)),
            ..Default::default()
        }
    );

    assert_eq!(
        theme.levels[&Level::Warning][&Element::LevelInner],
        Style {
            foreground: Some(Color::Palette(214)),
            modes: ModeSetDiff::new() - Mode::Faint,
            ..Default::default()
        }
    );
}

#[test]
fn test_v1_empty() {
    let theme = theme("v1-empty");

    assert_eq!(
        theme.elements[&Element::Level],
        Style {
            modes: Mode::Faint.into(),
            ..Default::default()
        }
    );

    assert_eq!(
        theme.elements[&Element::LevelInner],
        Style {
            modes: ModeSetDiff::new() - Mode::Faint,
            ..Default::default()
        }
    );

    assert_eq!(
        theme.levels[&Level::Warning][&Element::Level],
        Style {
            modes: Mode::Faint.into(),
            ..Default::default()
        }
    );

    assert_eq!(
        theme.levels[&Level::Warning][&Element::LevelInner],
        Style {
            foreground: Some(Color::Plain(PlainColor::Yellow)),
            modes: ModeSetDiff::new() - Mode::Faint,
            ..Default::default()
        }
    );
}

#[test]
fn test_file_format_parse_errors() {
    let yaml_result = Theme::load(&dirs(), "malformed.yaml");
    assert!(yaml_result.is_err(), "Malformed YAML should produce an error");
    let yaml_err = yaml_result.unwrap_err();
    let yaml_msg = yaml_err.to_string();
    assert!(
        yaml_msg.contains("malformed.yaml") || yaml_msg.contains("YAML") || yaml_msg.contains("parse"),
        "YAML error should be descriptive, got: {}",
        yaml_msg
    );

    let toml_result = Theme::load(&dirs(), "malformed.toml");
    assert!(toml_result.is_err(), "Malformed TOML should produce an error");
    let toml_err = toml_result.unwrap_err();
    let toml_msg = toml_err.to_string();
    assert!(
        toml_msg.contains("malformed.toml") || toml_msg.contains("TOML") || toml_msg.contains("parse"),
        "TOML error should be descriptive, got: {}",
        toml_msg
    );

    let json_result = Theme::load(&dirs(), "malformed.json");
    assert!(json_result.is_err(), "Malformed JSON should produce an error");
    let json_err = json_result.unwrap_err();
    let json_msg = json_err.to_string();
    assert!(
        json_msg.contains("malformed.json") || json_msg.contains("JSON") || json_msg.contains("parse"),
        "JSON error should be descriptive, got: {}",
        json_msg
    );
}

#[test]
fn test_unsupported_theme_version() {
    let result = Theme::load(&dirs(), "test-unsupported-version");
    assert!(result.is_err());
}

#[test]
fn test_v0_level_override_with_invalid_mode_prefix() {
    let result = Theme::load(&dirs(), "test-v0-level-invalid-mode");
    assert!(result.is_err());
}

#[test]
fn test_v0_element_with_invalid_mode_prefix() {
    let result = Theme::load(&dirs(), "test-v0-element-invalid-mode");
    assert!(result.is_err());
}

#[test]
fn test_invalid_style_base_deserialization() {
    let result = Theme::load(&dirs(), "test-invalid-style-base");
    assert!(result.is_err());
}

#[test]
fn test_style_base_deserialization_single_string() {
    let theme = raw_theme("test-base-single");
    let secondary = theme.styles.get(&Role::Secondary);
    assert!(secondary.is_some());
    assert!(!secondary.unwrap().base.is_empty());
}

#[test]
fn test_style_base_visitor_expecting() {
    let result = Theme::load(&dirs(), "test-invalid-style-base");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(!err_msg.is_empty());
}

#[test]
fn test_v1_strict_unknown_key_rejected() {
    let result = load_raw_theme("v1-unknown-key");

    assert!(
        result.is_err(),
        "v1 theme with unknown key should fail strict validation"
    );

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    assert!(
        err_msg.contains("unknown") || err_msg.contains("field"),
        "Error message should indicate unknown field, got: {}",
        err_msg
    );
}

#[test]
fn test_v1_strict_unknown_enum_variant_rejected() {
    let result = load_raw_theme("v1-unknown-role");

    assert!(
        result.is_err(),
        "v1 theme with unknown Role variant should fail strict validation"
    );

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    assert!(
        err_msg.contains("unknown") || err_msg.contains("variant") || err_msg.contains("future-role"),
        "Error message should indicate unknown enum variant, got: {}",
        err_msg
    );
}

#[test]
fn test_v1_schema_field_accepted() {
    let result = load_raw_theme("v1-with-schema");

    assert!(
        result.is_ok(),
        "v1 theme with $schema field should be accepted, got error: {:?}",
        result.err()
    );

    let theme = result.unwrap();
    let resolved = theme.resolve();
    assert!(resolved.is_ok(), "Theme with $schema should resolve successfully");

    let resolved = resolved.unwrap();
    assert_eq!(resolved.elements.len(), 3, "Should have 3 elements after resolution");
}
