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
    use themecfg::{Color, RGB, ResolvedStyle};

    let theme_style = ResolvedStyle::new().foreground(Some(Color::RGB(RGB(255, 128, 64))));

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

#[test]
fn test_boolean_merge_timing_with_level_overrides() {
    // Test FR-024: Boolean active merge timing relative to level-specific overrides
    // Question: Does level override to `boolean` affect boolean-true/false at that level?
    //
    // Current implementation: Boolean merge happens in StylePack::load() AFTER
    // level merging, so level overrides to `boolean` DO affect the variants.
    //
    // This test loads a theme file that has level-specific overrides to `boolean`
    // and verifies the theme loads successfully. The actual merge behavior happens
    // in StylePack::load() which is called during Theme::from(themecfg::Theme).

    use crate::appdirs::AppDirs;
    use std::path::PathBuf;

    let app_dirs = AppDirs {
        config_dir: PathBuf::from("src/testing/assets"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    };

    // Load the theme that has level-specific boolean overrides
    let cfg = themecfg::Theme::load(&app_dirs, "v0-boolean-level-override").unwrap();
    let theme = Theme::from(&cfg);

    // This test documents the current behavior:
    // The boolean merge happens AFTER level merging in StylePack::load(),
    // so error level's override to `boolean` DOES affect boolean-true/false.
    //
    // At error level in the test theme:
    // - boolean: foreground=#ff00ff (magenta), background=#110011, modes=[italic, underline]
    // - boolean-true: should inherit background and modes from error level's boolean
    //   but keep base boolean-true foreground (#00ffff) since no error-level override
    // - boolean-false: has explicit error-level override foreground=#ffaaaa
    //   should inherit background and modes from error level's boolean
    //
    // This is the CURRENT implementation behavior. Whether this is the INTENDED
    // spec behavior is unclear - FR-024 doesn't explicitly address level overrides.

    // Verify theme was created successfully
    let mut buf = Vec::new();
    theme.apply(&mut buf, &Some(Level::Error), |s| {
        s.element(Element::BooleanTrue, |s| s.batch(|buf| buf.extend_from_slice(b"true")));
    });
    assert!(!buf.is_empty());
}
