use super::super::{Color, Merge, MergeFlags, Mode, ModeSetDiff, PlainColor, RawStyle, RawTheme, StyleBase, Version};

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
