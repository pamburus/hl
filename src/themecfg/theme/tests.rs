use std::str::FromStr;

use strum::IntoEnumIterator;

use crate::level::Level;

use super::super::{
    Color, Element, Error, Format, Mode, ModeSetDiff, PlainColor, RGB, Style, Tag, Theme, Version,
    tests::{dirs, load_raw_theme_unmerged, theme},
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
    let result = load_raw_theme_unmerged("test-unknown-elements.toml");

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
    let result = load_raw_theme_unmerged("test-unknown-elements.json");

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
    let result = load_raw_theme_unmerged("test-unknown-elements.yaml");

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
