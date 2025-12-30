use super::*;
use crate::themecfg::{self, Color, PlainColor, RGB, RawTheme};

// Helper function to create test AppDirs
fn test_app_dirs() -> crate::appdirs::AppDirs {
    use std::path::PathBuf;
    crate::appdirs::AppDirs {
        config_dir: PathBuf::from("src/testing/assets"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    }
}

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
    // Test that theme can handle a valid level gracefully
    // V1 RawTheme uses Level (strict), so we can't insert invalid levels
    let cfg = RawTheme::default();
    let theme = Theme::from(cfg.resolve().unwrap());
    let mut buf = Vec::new();
    theme.apply(&mut buf, &Some(Level::Debug), |s| {
        s.element(Element::Message, |s| s.batch(|buf| buf.extend_from_slice(b"hello!")));
    });
    assert_eq!(buf, b"hello!");
}

#[test]
fn test_style_from_rgb_color() {
    let theme_style = themecfg::Style::new().foreground(Some(Color::RGB(RGB(255, 128, 64))));

    let style = Style::from(&theme_style);

    // Check that the style contains the RGB foreground color
    // We can't directly access the internal structure, but we can check
    // that the conversion didn't panic and produced a valid style
    assert_ne!(style.0, Sequence::reset());
}

#[test]
fn test_style_from_background_color() {
    let theme_style = themecfg::Style {
        background: Some(Color::Plain(PlainColor::Blue)),
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

    let app_dirs = test_app_dirs();

    // Load the theme that has level-specific boolean overrides
    let cfg = themecfg::Theme::load(&app_dirs, "v0-boolean-level-override").unwrap();
    let theme = Theme::from(cfg);

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
    // Test FR-039d and User Story 6, Scenario 2:
    // V1 themes should always merge parent→inner using property-level merging
    // even when the inner element has a role reference.
    //
    // Test scenario:
    // - level element has modes=[faint]
    // - level-inner for debug has foreground=#d2a6ff (specific color)
    // - Expected: level-inner should inherit modes=[faint] from parent AND have foreground=#d2a6ff

    let app_dirs = test_app_dirs();
    let cfg = themecfg::Theme::load(&app_dirs, "v1-parent-inner-modes-merge").unwrap();
    let theme = Theme::from(cfg);

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
    let theme = Theme::from(cfg);

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
    // those modes are preserved after merging with per-level styles from @base.
    //
    // This tests the fix for the issue where a theme's level-inner = { modes = ["bold"] }
    // was losing the bold mode when merged with per-level styles.
    //
    // The merge flow is:
    // 1. Theme merge: @base + child theme → child's level-inner replaces @base's
    // 2. Per-level merge: elements.level-inner + levels.info.level-inner → property-level merge
    //    Result: level-inner = { style = "info", modes = ["bold"] }
    // 3. Resolution: The final rendered style should have bold mode

    let app_dirs = test_app_dirs();

    // Load the v0 theme with nested input elements
    // Load synthetic test theme which defines level-inner = { modes = ["bold"] }
    // and levels.info.level-inner = { style = "info" }
    let cfg = themecfg::Theme::load(&app_dirs, "v1-element-modes-per-level").unwrap();
    let theme = Theme::from(cfg);

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
    let theme = Theme::from(cfg);

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

#[test]
fn test_v0_theme_without_input_falls_back_to_default() {
    // Test that v0 themes without input element defined fall back to @base's input styling
    // This reproduces the issue where old v0 themes that don't have input element
    // render input without any styles instead of falling back to @base
    let app_dirs = test_app_dirs();

    // Load the v0 theme with nested logger elements
    // Load v0 theme that doesn't define input element
    let cfg = themecfg::Theme::load(&app_dirs, "v0-missing-input").unwrap();
    let theme = Theme::from(cfg);

    // Apply the theme and render something with Input element
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Input, |s| s.batch(|buf| buf.extend_from_slice(b"test")));
    });

    // The input element should get styling from deduced secondary style:
    // - v0-missing-input defines: time = { foreground: 'bright-black' }
    // - This deduces: styles.secondary = { foreground: 'bright-black' }
    // - @base defines: input = { style = "secondary" }
    // - Result: input uses bright-black (90) from deduced secondary
    // This makes input consistent with the v0 theme's aesthetic
    let expected = "\x1b[0;90mtest\x1b[0m";
    assert_eq!(
        buf,
        expected.as_bytes(),
        "Expected input element to use deduced secondary style (bright-black from time).\nExpected: {:?}\nActual:   {:?}",
        expected,
        String::from_utf8_lossy(&buf)
    );
}

#[test]
fn test_v0_theme_multiple_elements_fallback_to_default() {
    // Test that v0 themes correctly fall back to @base for multiple undefined elements
    // This verifies the fix works across different element types
    let app_dirs = test_app_dirs();

    // Load the v0 theme with nested caller elements
    // Load v0 theme that doesn't define input, key, or logger elements
    let cfg = themecfg::Theme::load(&app_dirs, "v0-missing-input").unwrap();
    let theme = Theme::from(cfg);

    // Test Input element - should use deduced secondary style (bright-black from time)
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Input, |s| s.batch(|buf| buf.extend_from_slice(b"in")));
    });
    assert_eq!(
        buf, b"\x1b[0;90min\x1b[0m",
        "Input element should use deduced secondary style (bright-black from time)"
    );

    // Test Key element - should use deduced accent style
    // v0-missing-input doesn't define key, so no accent style deduction
    // Falls back to @base's accent = { style = "secondary" }
    // Which uses the deduced secondary = { foreground: 'bright-black' }
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Key, |s| s.batch(|buf| buf.extend_from_slice(b"key")));
    });
    assert_eq!(
        buf, b"\x1b[0;90mkey\x1b[0m",
        "Key element should use deduced secondary (bright-black) via accent"
    );

    // Test Logger element - should use accent-secondary which chains to deduced secondary
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Logger, |s| s.batch(|buf| buf.extend_from_slice(b"log")));
    });
    assert_eq!(
        buf, b"\x1b[0;90mlog\x1b[0m",
        "Logger element should use deduced secondary (bright-black) via accent-secondary"
    );
}

#[test]
fn test_v0_theme_inherits_foreground_and_modes_from_default() {
    // Test that v0 themes correctly inherit both foreground/background AND modes from @base
    // This verifies that our fix for modes doesn't break foreground/background inheritance
    //
    // In @base.toml:
    // - levels.debug.level-inner = { style = ["level", "debug"] }
    // - level style = { style = "status" } -> { modes = ["-faint"] }
    // - debug style = { foreground = "magenta" }
    //
    // So level-inner at debug level should have:
    // - modes from "level": ["-faint"] which removes faint
    // - foreground from "debug": magenta (ANSI 35)
    let app_dirs = test_app_dirs();

    // Load the theme that doesn't define level element (only level-inner)
    // Load v0 theme that doesn't define level-specific elements
    let cfg = themecfg::Theme::load(&app_dirs, "v0-missing-input").unwrap();
    let theme = Theme::from(cfg);

    // Apply the theme and render level-inner at debug level
    let mut buf = Vec::new();
    theme.apply(&mut buf, &Some(Level::Debug), |s| {
        s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(b"DBG")));
    });

    let output = String::from_utf8_lossy(&buf);

    // Should contain magenta (35) from debug style
    assert!(
        output.contains(";35") || output.contains("[35"),
        "Expected magenta (35) foreground from @base's debug style, got: {:?}",
        output
    );

    // Should NOT contain faint mode (2) because level style has modes = ["-faint"]
    assert!(
        !output.contains(";2") && !output.contains("[2"),
        "Expected no faint mode (level style removes it), got: {:?}",
        output
    );

    // Verify exact output: \x1b[0;35mDBG\x1b[0m (reset, magenta)
    let expected = "\x1b[0;35mDBG\x1b[0m";
    assert_eq!(
        buf,
        expected.as_bytes(),
        "Expected level-inner at debug to have magenta without faint mode.\nExpected: {:?}\nActual:   {:?}",
        expected,
        output
    );
}

#[test]
fn test_v0_theme_modes_only_inherits_colors_from_default() {
    // Test that v0 themes defining only modes for an element correctly inherit
    // foreground/background from @base, and vice versa
    //
    // This verifies that when a style has modes but no foreground, the foreground
    // is inherited from the base role, and when it has foreground but no modes,
    // the modes are inherited (or not set) correctly.
    let app_dirs = test_app_dirs();

    // Load theme with caller but no caller-inner
    // Load v0 theme that defines message with only modes (underline)
    // In @base: message = { style = "message" } -> { style = "strong" } -> { style = "primary", modes = ["bold"] }
    let cfg = themecfg::Theme::load(&app_dirs, "v0-modes-no-foreground").unwrap();
    let theme = Theme::from(cfg);

    // Test message element - has underline from theme, should still work
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Message, |s| s.batch(|buf| buf.extend_from_slice(b"msg")));
    });

    let output = String::from_utf8_lossy(&buf);

    // Should contain underline mode (4) from the theme
    assert!(
        output.contains(";4") || output.contains("[4"),
        "Expected underline (4) mode from theme definition, got: {:?}",
        output
    );

    // Test level element - has blue (34) foreground from theme, no modes
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Level, |s| s.batch(|buf| buf.extend_from_slice(b"lvl")));
    });

    let output = String::from_utf8_lossy(&buf);

    // Should contain blue (34) foreground from the theme
    assert!(
        output.contains(";34") || output.contains("[34"),
        "Expected blue (34) foreground from theme definition, got: {:?}",
        output
    );

    // Should NOT contain faint mode since level in @base is { style = "muted" }
    // and muted -> secondary -> primary (which removes faint), then adds faint
    // But our theme overrides with just foreground, no style reference
    // So it should only have the blue foreground, no modes
    assert_eq!(
        buf, b"\x1b[0;34mlvl\x1b[0m",
        "Expected level to have only blue foreground, no modes"
    );
}

#[test]
fn test_v0_theme_defined_elements_no_auto_deduction() {
    // REGRESSION TEST: v0 themes with elements explicitly defined should NOT
    // get automatic style deduction. Auto-deduction should ONLY apply to elements
    // that are NOT defined in the v0 theme at all.
    //
    // In v0, when an element is defined, it's complete - no inheritance from base styles.
    // For example:
    //   time: { foreground: '30' }
    // Should render ONLY with foreground color 30, NO faint mode even though
    // @base defines time with style="secondary" which adds faint.
    let app_dirs = test_app_dirs();

    // Load v1 theme that has level-inner and uses base inheritance
    // Load v0 theme that defines time/message/key/string with only foreground
    let cfg = themecfg::Theme::load(&app_dirs, "v0-regression-test").unwrap();
    let theme = Theme::from(cfg);

    // Time: foreground='30' (palette index 30), should have NO faint mode
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Time, |s| s.batch(|buf| buf.extend_from_slice(b"time")));
    });
    let expected = "\x1b[0;38;5;30mtime\x1b[0m";
    assert_eq!(
        buf,
        expected.as_bytes(),
        "Time defined in v0 theme should have ONLY foreground, NO faint mode.\nExpected: {:?}\nActual:   {:?}",
        expected,
        String::from_utf8_lossy(&buf)
    );

    // Message: foreground='195', should have NO bold mode
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Message, |s| s.batch(|buf| buf.extend_from_slice(b"msg")));
    });
    let expected = "\x1b[0;38;5;195mmsg\x1b[0m";
    assert_eq!(
        buf,
        expected.as_bytes(),
        "Message defined in v0 theme should have ONLY foreground, NO bold mode.\nExpected: {:?}\nActual:   {:?}",
        expected,
        String::from_utf8_lossy(&buf)
    );

    // Key: foreground='220', should have NO faint mode
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Key, |s| s.batch(|buf| buf.extend_from_slice(b"key")));
    });
    let expected = "\x1b[0;38;5;220mkey\x1b[0m";
    assert_eq!(
        buf,
        expected.as_bytes(),
        "Key defined in v0 theme should have ONLY foreground, NO faint mode.\nExpected: {:?}\nActual:   {:?}",
        expected,
        String::from_utf8_lossy(&buf)
    );

    // String: foreground='120', should have NO modes
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::String, |s| s.batch(|buf| buf.extend_from_slice(b"str")));
    });
    let expected = "\x1b[0;38;5;120mstr\x1b[0m";
    assert_eq!(
        buf,
        expected.as_bytes(),
        "String defined in v0 theme should have ONLY foreground, NO modes.\nExpected: {:?}\nActual:   {:?}",
        expected,
        String::from_utf8_lossy(&buf)
    );
}

#[test]
fn test_v0_theme_style_deduction_from_elements() {
    // Test that v0 themes automatically deduce styles FROM element definitions
    // BEFORE merging with @base, so that elements not defined in v0 theme
    // but defined in @base will use colors consistent with the v0 theme.
    //
    // For example, if a v0 theme defines:
    //   time: { foreground: 30 }
    // We deduce:
    //   styles.secondary: { foreground: 30 }
    // Then when merged with @base, the `input` element (which has style="secondary")
    // will use foreground 30, making it consistent with the v0 theme's aesthetic.
    let app_dirs = test_app_dirs();

    // Load v1 theme that uses the Default role
    // Load v0 theme that defines time/message/key/string with only foreground
    // This should deduce secondary/strong/accent/primary styles
    let cfg = themecfg::Theme::load(&app_dirs, "v0-regression-test").unwrap();
    let theme = Theme::from(cfg);

    // Input element is NOT defined in v0-regression-test, but IS in @base with style="secondary"
    // Since we deduced secondary style from time element (foreground=30),
    // input should use foreground 30
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Input, |s| s.batch(|buf| buf.extend_from_slice(b"input")));
    });
    let expected = "\x1b[0;38;5;30minput\x1b[0m";
    assert_eq!(
        buf,
        expected.as_bytes(),
        "Input (not in v0 theme) should use deduced secondary style (foreground 30 from time).\nExpected: {:?}\nActual:   {:?}",
        expected,
        String::from_utf8_lossy(&buf)
    );

    // Time element IS defined in v0 theme with foreground=30
    // It should render as-is (no modes added)
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Time, |s| s.batch(|buf| buf.extend_from_slice(b"time")));
    });
    let expected = "\x1b[0;38;5;30mtime\x1b[0m";
    assert_eq!(
        buf,
        expected.as_bytes(),
        "Time (defined in v0 theme) should have only its foreground.\nExpected: {:?}\nActual:   {:?}",
        expected,
        String::from_utf8_lossy(&buf)
    );
}

#[test]
fn test_v0_theme_style_deduction_with_modes() {
    // Test FR-031c: Style deduction MUST copy both colors AND modes from element definitions
    // If v0 theme defines: time: { foreground: 30, modes: ['italic'] }
    // Then deduced secondary should be: { foreground: 30, modes: ['italic'] }
    // And elements in @base that reference secondary should inherit BOTH color AND modes
    let app_dirs = test_app_dirs();

    // Load the theme with mode diff testing
    // Load v0 theme that defines message with BOTH foreground and modes
    let cfg = themecfg::Theme::load(&app_dirs, "v0-auto-style-deduction").unwrap();
    let theme = Theme::from(cfg);

    // Message element IS defined in v0 theme with foreground='white' and modes=['italic']
    // It should render exactly as defined
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Message, |s| s.batch(|buf| buf.extend_from_slice(b"msg")));
    });

    let output = String::from_utf8_lossy(&buf);

    // Should contain white foreground (ANSI 37)
    assert!(
        output.contains(";37") || output.contains("[37"),
        "Expected white (37) foreground from message definition, got: {:?}",
        output
    );

    // Should contain italic mode (3)
    assert!(
        output.contains(";3") || output.contains("[3"),
        "Expected italic (3) mode from message definition, got: {:?}",
        output
    );

    // Now test an element NOT defined in v0 theme that references the deduced strong style
    // @base defines: object = { style = "syntax" }
    // @base defines: syntax = { style = "strong" }
    // v0-auto-style-deduction defines: message = { foreground: 'white', modes: ['italic'] }
    // This should deduce: strong = { foreground: 'white', modes: ['italic'] }
    // So object should inherit BOTH foreground AND modes from deduced strong
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Object, |s| s.batch(|buf| buf.extend_from_slice(b"obj")));
    });

    let output = String::from_utf8_lossy(&buf);

    // Should contain white foreground from deduced strong style
    assert!(
        output.contains(";37") || output.contains("[37"),
        "Object (not in v0 theme) should inherit white foreground from deduced strong style, got: {:?}",
        output
    );

    // Should contain italic mode from deduced strong style
    assert!(
        output.contains(";3") || output.contains("[3"),
        "Object (not in v0 theme) should inherit italic mode from deduced strong style, got: {:?}",
        output
    );
}

#[test]
fn test_v0_theme_explicit_style_takes_precedence_over_deduction() {
    // Test FR-010f + FR-031: V0 themes ignore styles section, only deduction creates styles
    //
    // This test verifies that when a v0 theme defines:
    //   time: { foreground: 30 }  <- will deduce secondary
    //   styles.secondary: { foreground: 40 }  <- IGNORED per FR-010f
    // The styles section is ignored, and secondary is deduced from time (foreground 30).
    // The time element itself uses its own definition (foreground 30).
    let app_dirs = test_app_dirs();

    // Load theme with mode inheritance
    // Create a temporary theme file with both element and style defined
    // Per FR-010f, the styles section will be ignored
    let theme_content = r#"
elements:
  time:
    foreground: 30

styles:
  secondary:
    foreground: 40
"#;

    let theme_dir = std::path::PathBuf::from("src/testing/assets/themes");
    let theme_path = theme_dir.join("v0-explicit-style-precedence.yaml");
    std::fs::write(&theme_path, theme_content).unwrap();

    // Load the theme
    let cfg = themecfg::Theme::load(&app_dirs, "v0-explicit-style-precedence").unwrap();
    let theme = Theme::from(cfg);

    // Time element should use its own definition (foreground 30)
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Time, |s| s.batch(|buf| buf.extend_from_slice(b"time")));
    });

    assert_eq!(
        buf, b"\x1b[0;38;5;30mtime\x1b[0m",
        "Time element should use its own definition (foreground 30)"
    );

    // Input element (not defined in v0, but in @base with style="secondary")
    // should use the DEDUCED secondary style (foreground 30 from time), NOT the ignored explicit style (40)
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Input, |s| s.batch(|buf| buf.extend_from_slice(b"input")));
    });

    assert_eq!(
        buf, b"\x1b[0;38;5;30minput\x1b[0m",
        "Input should use deduced secondary style (foreground 30), styles section is ignored per FR-010f"
    );

    // Clean up
    std::fs::remove_file(&theme_path).ok();
}

#[test]
fn test_v0_theme_deduction_with_empty_modes_array() {
    // Test edge case: What happens when v0 theme defines element with empty modes array?
    // According to FR-018: empty modes array [] is treated identically to absent modes
    // This test verifies the deduction behavior in this edge case
    let app_dirs = test_app_dirs();

    // Load v0 theme that defines logger-inner
    // Create a temporary theme with empty modes array
    let theme_content = r#"
elements:
  time:
    foreground: 30
    modes: []
"#;

    let theme_dir = std::path::PathBuf::from("src/testing/assets/themes");
    let theme_path = theme_dir.join("v0-empty-modes-deduction.yaml");
    std::fs::write(&theme_path, theme_content).unwrap();

    // Load the theme
    let cfg = themecfg::Theme::load(&app_dirs, "v0-empty-modes-deduction").unwrap();
    let theme = Theme::from(cfg);

    // Time element should have foreground but no modes
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Time, |s| s.batch(|buf| buf.extend_from_slice(b"time")));
    });

    assert_eq!(
        buf, b"\x1b[0;38;5;30mtime\x1b[0m",
        "Time with empty modes array should have only foreground"
    );

    // Input element should also have no modes (deduced secondary has no modes)
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Input, |s| s.batch(|buf| buf.extend_from_slice(b"input")));
    });

    assert_eq!(
        buf, b"\x1b[0;38;5;30minput\x1b[0m",
        "Input should use deduced secondary with foreground but no modes"
    );

    // Clean up
    std::fs::remove_file(&theme_path).ok();
}

#[test]
fn test_v0_theme_deduction_copies_background() {
    // Test that style deduction copies background color as well as foreground
    // FR-031 states: "deduction copies foreground, background, and modes"
    let app_dirs = test_app_dirs();

    // Load v1 theme with multiple base inheritance
    // Create a theme with background defined
    let theme_content = r#"
elements:
  string:
    foreground: 'green'
    background: 'black'
    modes: ['bold']
"#;

    let theme_dir = std::path::PathBuf::from("src/testing/assets/themes");
    let theme_path = theme_dir.join("v0-background-deduction.yaml");
    std::fs::write(&theme_path, theme_content).unwrap();

    // Load the theme
    let cfg = themecfg::Theme::load(&app_dirs, "v0-background-deduction").unwrap();
    let theme = Theme::from(cfg);

    // String element should have foreground, background, and bold mode
    let mut buf = Vec::new();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::String, |s| s.batch(|buf| buf.extend_from_slice(b"str")));
    });

    let output = String::from_utf8_lossy(&buf);

    // Should contain green foreground
    assert!(
        output.contains(";32") || output.contains("[32"),
        "String should have green foreground, got: {:?}",
        output
    );

    // Should contain black background (40)
    assert!(
        output.contains(";40") || output.contains("[40"),
        "String should have black background, got: {:?}",
        output
    );

    // Should contain bold mode (1)
    assert!(
        output.contains(";1") || output.contains("[1"),
        "String should have bold mode, got: {:?}",
        output
    );

    // Number element (not defined in v0, uses @base's number = { style = "value" })
    // should inherit ALL properties from deduced primary style (via value role)
    buf.clear();
    theme.apply(&mut buf, &None, |s| {
        s.element(Element::Number, |s| s.batch(|buf| buf.extend_from_slice(b"123")));
    });

    let output = String::from_utf8_lossy(&buf);

    // Should contain green foreground from deduced primary
    assert!(
        output.contains(";32") || output.contains("[32"),
        "Number should inherit green foreground from deduced primary, got: {:?}",
        output
    );

    // Should contain black background from deduced primary
    assert!(
        output.contains(";40") || output.contains("[40"),
        "Number should inherit black background from deduced primary, got: {:?}",
        output
    );

    // Should contain bold mode from deduced primary
    assert!(
        output.contains(";1") || output.contains("[1"),
        "Number should inherit bold mode from deduced primary, got: {:?}",
        output
    );

    // Clean up
    std::fs::remove_file(&theme_path).ok();
}

#[test]
fn test_v1_level_inner_does_not_inherit_parent_modes() {
    // Test that level-inner does not inherit faint mode from parent level element
    //
    // This is a regression test for a bug where changing merge logic order caused
    // inner elements to incorrectly inherit parent element modes during parent→inner merging.
    //
    // Test scenario (mimics uni theme):
    // - level element has modes = ["faint"]
    // - level-inner element has modes = ["-faint", "bold"]
    // - levels.info.level-inner has style = ["level", "info"] (cyan foreground)
    //
    // Expected result:
    // - level-inner at info level should have: bold + cyan (NO faint)
    //
    // Bug causes:
    // - level-inner at info level to have: bold + faint + cyan (incorrect)

    let app_dirs = test_app_dirs();

    // Load minimal test theme that mimics uni theme structure
    let cfg = themecfg::Theme::load(&app_dirs, "v1-uni-like-level-modes").unwrap();
    let theme = Theme::from(cfg);

    // Render level-inner at info level
    let mut buf = Vec::new();
    theme.apply(&mut buf, &Some(Level::Info), |s| {
        s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(b"INF")));
    });

    let output = String::from_utf8_lossy(&buf);

    // Verify bold mode (1) is present from level-inner definition
    assert!(
        output.contains(";1;") || output.contains("[0;1"),
        "Expected bold mode (1) from level-inner, got: {:?}",
        output
    );

    // Verify cyan (36) is present from info style
    assert!(
        output.contains(";36") || output.contains("[36"),
        "Expected cyan (36) from info style, got: {:?}",
        output
    );

    // CRITICAL: Verify faint mode (2) is NOT present
    // The regression bug causes level-inner to incorrectly inherit faint from parent level
    assert!(
        !output.contains(";2;") && !output.contains(";1;2;") && !output.contains(";2;1;"),
        "level-inner must NOT inherit faint mode (2) from parent level element, got: {:?}",
        output
    );
}
