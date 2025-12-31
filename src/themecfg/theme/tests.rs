use std::str::FromStr;

use strum::IntoEnumIterator;

use crate::level::Level;

use super::super::{
    Color, Element, Format, Mode, ModeSetDiff, PlainColor, RGB, Style, Tag, Theme,
    tests::{dirs, theme},
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
