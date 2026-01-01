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
    let theme_style = themecfg::ResolvedStyle {
        background: Some(themecfg::Color::Plain(themecfg::PlainColor::Blue)),
        ..Default::default()
    };

    let style = Style::from(&theme_style);
    assert_ne!(style.0, Sequence::reset());
}

#[test]
fn test_style_from_foreground_plain_color() {
    let theme_style = themecfg::ResolvedStyle {
        foreground: Some(themecfg::Color::Plain(themecfg::PlainColor::Red)),
        ..Default::default()
    };

    let style = Style::from(&theme_style);
    assert_ne!(style.0, Sequence::reset());
}

#[test]
fn test_style_from_background_rgb_color() {
    let theme_style = themecfg::ResolvedStyle {
        background: Some(themecfg::Color::RGB(themecfg::RGB(100, 150, 200))),
        ..Default::default()
    };

    let style = Style::from(&theme_style);
    assert_ne!(style.0, Sequence::reset());
}

#[test]
fn test_style_from_default_colors_ignored() {
    let theme_style = themecfg::ResolvedStyle {
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

#[test]
fn test_v1_parent_inner_property_level_merging() {
    // Test FR-037d and User Story 6, Scenario 2:
    // V1 themes should always merge parent→inner using property-level merging
    // even when the inner element has a role reference.
    //
    // Test scenario:
    // - level element has modes=[faint] (in ayu-dark-24 theme)
    // - level-inner for debug has foreground=#d2a6ff (specific color)
    // - Expected: level-inner should inherit modes=[faint] from parent AND have foreground=#d2a6ff

    use crate::appdirs::AppDirs;
    use std::path::PathBuf;

    let app_dirs = AppDirs {
        config_dir: PathBuf::from("etc/defaults"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    };

    // Load ayu-dark-24 which is a v1 theme
    let cfg = themecfg::Theme::load(&app_dirs, "ayu-dark-24").unwrap();
    let theme = Theme::from(&cfg);

    // Apply the theme and render something with level-inner at debug level
    let mut buf = Vec::new();
    theme.apply(&mut buf, &Some(Level::Debug), |s| {
        s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(b"DBG")));
    });

    // The buffer should contain ANSI codes that include both:
    // - The faint mode (2) from the parent level element
    // - The color code from the debug level-inner foreground
    let output = String::from_utf8_lossy(&buf);

    // Check that faint mode (2) is present in the output
    // ANSI faint mode is "\u{1b}[2m" or as part of combined codes like "\u{1b}[0;2m"
    assert!(
        output.contains(";2") || output.contains("[2"),
        "Expected faint mode to be inherited from parent level element, got: {}",
        output
    );
}

#[test]
fn test_v0_input_element_styling() {
    // Test that the Input element is correctly loaded and styled from v0 themes
    use crate::appdirs::AppDirs;
    use std::path::PathBuf;

    let app_dirs = AppDirs {
        config_dir: PathBuf::from("etc/defaults"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    };

    // Load classic theme (v0)
    let cfg = themecfg::Theme::load(&app_dirs, "classic").unwrap();
    let theme = Theme::from(&cfg);

    // Apply the theme and render something with Input element
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Input, |s| s.batch(|buf| buf.extend_from_slice(b"test")));
    });

    // The buffer should contain ANSI codes for bright-black (90)
    let output = String::from_utf8_lossy(&buf);

    // bright-black is ANSI code 90
    assert!(
        output.contains(";90") || output.contains("[90"),
        "Expected bright-black (90) color code for Input element, got: {:?}",
        output
    );
}

#[test]
fn test_v1_element_modes_preserved_after_per_level_merge() {
    // Test that when a v1 theme defines an element with modes (e.g., level-inner with bold),
    // those modes are preserved after merging with per-level styles from @default.
    //
    // This tests the fix for the issue where a theme's level-inner = { modes = ["bold"] }
    // was losing the bold mode when merged with per-level styles.
    //
    // The merge flow is:
    // 1. Theme merge: @default + child theme → child's level-inner replaces @default's
    // 2. Per-level merge: elements.level-inner + levels.info.level-inner → property-level merge
    //    Result: level-inner = { style = "info", modes = ["bold"] }
    // 3. Resolution: The final rendered style should have bold mode

    use crate::appdirs::AppDirs;
    use std::path::PathBuf;

    let app_dirs = AppDirs {
        config_dir: PathBuf::from("src/testing/assets"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    };

    // Load synthetic test theme which defines level-inner = { modes = ["bold"] }
    // and levels.info.level-inner = { style = "info" }
    let cfg = themecfg::Theme::load(&app_dirs, "v1-element-modes-per-level").unwrap();
    let theme = Theme::from(&cfg);

    // Apply the theme and render level-inner at info level
    let mut buf = Vec::new();
    theme.apply(&mut buf, &Some(Level::Info), |s| {
        s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(b"INF")));
    });

    let output = String::from_utf8_lossy(&buf);

    // Check that bold mode (1) is present in the output
    // ANSI bold mode is represented as "1" in the escape sequence
    // Expected output: \x1b[0;1;36mINF\x1b[0m (reset, bold, cyan)
    assert!(
        output.contains(";1;") || output.contains("[0;1"),
        "Expected bold mode (1) to be preserved from element definition after per-level merge, got: {:?}",
        output
    );

    // Also verify cyan (36) is present from the info style
    assert!(
        output.contains(";36") || output.contains("[36"),
        "Expected cyan (36) from info style, got: {:?}",
        output
    );
}

#[test]
fn test_v0_input_nested_styling() {
    // Test that v0 themes with `input` defined get nested styling scope behavior
    // where InputNumber inherits from Input via nested rendering scope
    use crate::appdirs::AppDirs;
    use std::path::PathBuf;

    let app_dirs = AppDirs {
        config_dir: PathBuf::from("etc/defaults"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    };

    // Load classic theme (v0) which only defines `input`, not `input-number`
    let cfg = themecfg::Theme::load(&app_dirs, "classic").unwrap();
    let theme = Theme::from(&cfg);

    // Render nested elements: Input containing InputNumber containing content
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Input, |s| {
            s.element(Element::InputNumber, |s| s.batch(|buf| buf.extend_from_slice(b"#0")))
        });
    });

    let output = String::from_utf8_lossy(&buf);

    // In v0, InputNumber should inherit from Input via nested styling scope
    // Since Input has bright-black (90), the nested content should also be bright-black
    assert!(
        output.contains(";90") || output.contains("[90"),
        "Expected bright-black (90) color for nested InputNumber element (inherited from Input), got: {:?}",
        output
    );
}
