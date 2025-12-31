use crate::level::Level;

use super::super::{
    Color, Element, Merge, MergeFlags, Mode, ModeSetDiff, PlainColor, RawStyle, RawTheme, StyleBase, Version,
    tests::raw_theme_unmerged,
};

#[test]
fn test_raw_theme_inner_mut() {
    let mut theme = RawTheme::default();
    theme.inner_mut().version = Version::new(1, 0);
    assert_eq!(theme.inner().version, Version::new(1, 0));
}

#[test]
fn test_raw_theme_into_inner() {
    let mut theme = RawTheme::default();
    theme.inner_mut().version = Version::new(1, 0);
    let inner = theme.into_inner();
    assert_eq!(inner.version, Version::new(1, 0));
}

#[test]
fn test_style_merge() {
    let base = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Bold.into(),
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: Some(Color::Plain(PlainColor::Blue)),
    };

    let patch = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Italic.into(),
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: None,
    };

    let result = base.clone().merged(&patch, MergeFlags::default());

    assert_eq!(result.modes, ModeSetDiff::from(Mode::Bold | Mode::Italic));
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Green)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Blue)));

    let patch = RawStyle {
        background: Some(Color::Plain(PlainColor::Green)),
        ..Default::default()
    };

    let result = base.clone().merged(&patch, MergeFlags::default());

    assert_eq!(result.modes, ModeSetDiff::from(Mode::Bold));
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Green)));
}

#[test]
fn test_v0_unknown_elements_ignored() {
    let theme = raw_theme_unmerged("v0-unknown-elements").resolve().unwrap();

    assert_eq!(theme.elements.len(), 1);
    assert!(theme.elements.contains_key(&Element::Message));
}

#[test]
fn test_v0_unknown_level_names_ignored() {
    let theme = raw_theme_unmerged("v0-unknown-levels");

    assert!(theme.levels.contains_key(&Level::Error), "Should have error level");

    assert_eq!(
        theme.levels.len(),
        1,
        "Should have only 1 valid level (unknown levels dropped during v0->v1 conversion)"
    );
}
