use super::*;

#[test]
fn test_theme() {
    let theme = Theme::none();
    let mut buf = Vec::new();
    theme.apply(&mut buf, &Some(Level::Debug), |s| {
        s.element(Element::Message, |s| s.batch(|buf| buf.extend_from_slice(b"hello!")));
    });
    assert_eq!(buf, b"hello!");
}

#[test]
fn test_unknown_level() {
    let mut cfg = themecfg::Theme::default();
    cfg.levels
        .insert(InfallibleLevel::Invalid("unknown".to_string()), Default::default());
    let theme = Theme::from(&cfg);
    let mut buf = Vec::new();
    theme.apply(&mut buf, &Some(Level::Debug), |s| {
        s.element(Element::Message, |s| s.batch(|buf| buf.extend_from_slice(b"hello!")));
    });
    assert_eq!(buf, b"hello!");
}

#[test]
fn test_style_from_rgb_color() {
    let theme_style = themecfg::Style {
        foreground: Some(themecfg::Color::RGB(themecfg::RGB(255, 128, 64))),
        ..Default::default()
    };

    let style = Style::from(&theme_style);

    // Check that the style contains the RGB foreground color
    // We can't directly access the internal structure, but we can check
    // that the conversion didn't panic and produced a valid style
    assert_ne!(style.0, Sequence::reset());
}

#[test]
fn test_style_from_background_color() {
    let theme_style = themecfg::Style {
        background: Some(themecfg::Color::Plain(themecfg::PlainColor::Blue)),
        ..Default::default()
    };

    let style = Style::from(&theme_style);
    assert_ne!(style.0, Sequence::reset());
}

#[test]
fn test_style_from_foreground_plain_color() {
    let theme_style = themecfg::Style {
        foreground: Some(themecfg::Color::Plain(themecfg::PlainColor::Red)),
        ..Default::default()
    };

    let style = Style::from(&theme_style);
    assert_ne!(style.0, Sequence::reset());
}

#[test]
fn test_style_from_background_rgb_color() {
    let theme_style = themecfg::Style {
        background: Some(themecfg::Color::RGB(themecfg::RGB(100, 150, 200))),
        ..Default::default()
    };

    let style = Style::from(&theme_style);
    assert_ne!(style.0, Sequence::reset());
}

#[test]
fn test_style_from_default_colors_ignored() {
    let theme_style = themecfg::Style {
        foreground: Some(themecfg::Color::Plain(themecfg::PlainColor::Default)),
        background: Some(themecfg::Color::Plain(themecfg::PlainColor::Default)),
        ..Default::default()
    };

    let style = Style::from(&theme_style);
    assert_eq!(style.0, Sequence::reset());
}
