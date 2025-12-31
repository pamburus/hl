// std imports
use std::{collections::HashMap, path::PathBuf};

// third-party imports

use yaml_peg::serde as yaml;

// local imports
use crate::{appdirs::AppDirs, level::Level};

// relative imports
use super::*;

// ---

// Helper function to create test AppDirs
pub(crate) fn dirs() -> AppDirs {
    AppDirs {
        config_dir: PathBuf::from("src/testing/assets/fixtures"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    }
}

pub(crate) fn theme(name: &str) -> Theme {
    Theme::load(&dirs(), name).unwrap()
}

pub(crate) fn raw_theme(name: &str) -> RawTheme {
    Theme::load_raw(&dirs(), name).unwrap()
}

pub(crate) fn load_raw_theme_unmerged(name: &str) -> Result<RawTheme> {
    Theme::load_from(&Theme::themes_dir(&dirs()), name)
}

pub(crate) fn raw_theme_unmerged(name: &str) -> RawTheme {
    load_raw_theme_unmerged(name).unwrap()
}

pub(crate) fn load_yaml_fixture<T>(path: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    let content = std::fs::read_to_string(PathBuf::from("src/testing/assets").join(path)).unwrap();
    let items: Vec<T> = yaml::from_str(&content).unwrap();
    items.into_iter().next().unwrap()
}

// Helper for displaying serializable types in tests
struct SerdeDisplay<'a, T>(&'a T);

impl<'a, T: serde::Serialize + std::fmt::Debug> std::fmt::Display for SerdeDisplay<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_plain::to_string(self.0) {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "{:?}", self.0),
        }
    }
}

fn display<T: serde::Serialize + std::fmt::Debug>(value: &T) -> SerdeDisplay<'_, T> {
    SerdeDisplay(value)
}

// Helper function to create ModeSetDiff from a list of modes (v0 semantics - only adds, no removes)
pub(crate) fn modes(modes: &[Mode]) -> ModeSetDiff {
    let mut mode_set = ModeSet::new();
    for &mode in modes {
        mode_set.insert(mode);
    }
    ModeSetDiff::from(mode_set)
}

#[test]
fn test_v0_parent_inner_blocking_all_pairs() {
    // Test that all 5 parent-inner pairs are blocked when parent is defined in child theme
    // This verifies the complete blocking rule implementation
    let mut base = RawTheme::default();

    // Base has all 5 -inner elements
    base.elements.insert(Element::LevelInner, RawStyle::default());
    base.elements.insert(Element::LoggerInner, RawStyle::default());
    base.elements.insert(Element::CallerInner, RawStyle::default());
    base.elements.insert(Element::InputNumberInner, RawStyle::default());
    base.elements.insert(Element::InputNameInner, RawStyle::default());

    // Child theme has all 5 parent elements
    let mut child = RawTheme::default();
    child.elements.insert(Element::Level, RawStyle::default());
    child.elements.insert(Element::Logger, RawStyle::default());
    child.elements.insert(Element::Caller, RawStyle::default());
    child.elements.insert(Element::InputNumber, RawStyle::default());
    child.elements.insert(Element::InputName, RawStyle::default());

    // Merge
    let merged = base.merged(child);

    // All -inner elements should be blocked (removed)
    assert!(
        !merged.elements.contains_key(&Element::LevelInner),
        "level-inner should be blocked"
    );
    assert!(
        !merged.elements.contains_key(&Element::LoggerInner),
        "logger-inner should be blocked"
    );
    assert!(
        !merged.elements.contains_key(&Element::CallerInner),
        "caller-inner should be blocked"
    );
    assert!(
        !merged.elements.contains_key(&Element::InputNumberInner),
        "input-number-inner should be blocked"
    );
    assert!(
        !merged.elements.contains_key(&Element::InputNameInner),
        "input-name-inner should be blocked"
    );

    // All parent elements should be present
    assert!(merged.elements.contains_key(&Element::Level), "level should be present");
    assert!(
        merged.elements.contains_key(&Element::Logger),
        "logger should be present"
    );
    assert!(
        merged.elements.contains_key(&Element::Caller),
        "caller should be present"
    );
    assert!(
        merged.elements.contains_key(&Element::InputNumber),
        "input-number should be present"
    );
    assert!(
        merged.elements.contains_key(&Element::InputName),
        "input-name should be present"
    );
}

#[test]
fn test_v0_level_section_blocking() {
    // Test that when child defines ANY element for a level, parent's entire level section is removed
    // FR-027: V0 level section blocking for backward compatibility
    let mut base = RawTheme::default();

    // Base theme has level sections with multiple elements
    let mut error_pack = v1::StylePack::default();
    error_pack.insert(
        Element::Message,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Red)),
            background: None,
            modes: Default::default(),
        },
    );
    error_pack.insert(
        Element::Level,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Blue)),
            background: None,
            modes: Default::default(),
        },
    );
    base.levels.insert(Level::Error, error_pack);

    let mut info_pack = v1::StylePack::default();
    info_pack.insert(
        Element::Message,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Green)),
            background: None,
            modes: Default::default(),
        },
    );
    base.levels.insert(Level::Info, info_pack);

    // Child theme defines just ONE element for error level (not info)
    let mut child = RawTheme::default();
    let mut child_error_pack = v1::StylePack::default();
    child_error_pack.insert(
        Element::Time,
        RawStyle {
            base: StyleBase::default(),
            modes: modes(&[Mode::Bold]),
            foreground: None,
            background: None,
        },
    );
    child.levels.insert(Level::Error, child_error_pack);

    // Merge
    let merged = base.merged(child);

    // Error level section should be completely replaced (base error elements removed)
    let error_level = &merged.levels[&Level::Error];
    assert!(error_level.contains_key(&Element::Time), "Child time should be present");
    assert!(
        !error_level.contains_key(&Element::Message),
        "Base error message should be blocked"
    );
    assert!(
        !error_level.contains_key(&Element::Level),
        "Base error level should be blocked"
    );

    // Info level section should remain (child didn't define it)
    let info_level = &merged.levels[&Level::Info];
    assert!(
        info_level.contains_key(&Element::Message),
        "Base info message should remain"
    );
}

#[test]
fn test_v0_multiple_blocking_rules_combined() {
    // Test that multiple blocking rules can trigger simultaneously
    // Parent-inner blocking + input blocking + level section blocking
    let mut base = RawTheme::default();

    // Base has parent-inner elements
    base.elements.insert(Element::LevelInner, RawStyle::default());
    base.elements.insert(Element::LoggerInner, RawStyle::default());

    // Base has input elements
    base.elements.insert(Element::InputNumber, RawStyle::default());
    base.elements.insert(Element::InputName, RawStyle::default());

    // Base has level sections
    let mut error_pack = v1::StylePack::default();
    error_pack.insert(Element::Message, RawStyle::default());
    base.levels.insert(Level::Error, error_pack);

    // Child triggers all blocking rules
    let mut child = RawTheme::default();
    child.elements.insert(Element::Level, RawStyle::default()); // Blocks level-inner
    child.elements.insert(Element::Logger, RawStyle::default()); // Blocks logger-inner
    child.elements.insert(Element::Input, RawStyle::default()); // Blocks input-number/input-name

    let mut child_error_pack = v1::StylePack::default();
    child_error_pack.insert(Element::Time, RawStyle::default());
    child.levels.insert(Level::Error, child_error_pack); // Blocks error section

    // Merge
    let merged = base.merged(child);

    // Verify all blocking happened
    assert!(
        !merged.elements.contains_key(&Element::LevelInner),
        "level-inner blocked by parent rule"
    );
    assert!(
        !merged.elements.contains_key(&Element::LoggerInner),
        "logger-inner blocked by parent rule"
    );
    assert!(
        !merged.elements.contains_key(&Element::InputNumber),
        "input-number blocked by input rule"
    );
    assert!(
        !merged.elements.contains_key(&Element::InputName),
        "input-name blocked by input rule"
    );

    let error_level = &merged.levels[&Level::Error];
    assert!(
        !error_level.contains_key(&Element::Message),
        "Base error message blocked by level section rule"
    );
    assert!(
        error_level.contains_key(&Element::Time),
        "Child error time should be present"
    );
}

#[test]
fn test_v1_no_blocking_rules() {
    // Test that v1 themes do NOT apply blocking rules (no ReplaceGroups flag)
    // Elements merge additively without blocking parent-inner pairs
    let mut base = RawTheme::default();
    base.inner_mut().version = Version { major: 1, minor: 0 };

    // Base has -inner elements
    base.elements.insert(
        Element::LevelInner,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Red)),
            background: None,
            modes: Default::default(),
        },
    );

    // Base has input elements
    base.elements.insert(Element::InputNumber, RawStyle::default());
    base.elements.insert(Element::InputName, RawStyle::default());

    // Base has level sections
    let mut error_pack = v1::StylePack::default();
    error_pack.insert(
        Element::Message,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Red)),
            background: None,
            modes: Default::default(),
        },
    );
    base.levels.insert(Level::Error, error_pack);

    // Child v1 theme defines parent elements
    let mut child = RawTheme::default();
    child.inner_mut().version = Version { major: 1, minor: 0 };
    child.elements.insert(Element::Level, RawStyle::default()); // Does NOT block level-inner in v1
    child.elements.insert(Element::Input, RawStyle::default()); // Does NOT block input-number in v1

    // Child defines error level element
    let mut child_error_pack = v1::StylePack::default();
    child_error_pack.insert(Element::Time, RawStyle::default());
    child.levels.insert(Level::Error, child_error_pack);

    // Merge
    let merged = base.merged(child);

    // In v1, no blocking should happen - elements should merge additively
    assert!(
        merged.elements.contains_key(&Element::LevelInner),
        "v1 should NOT block level-inner"
    );
    assert!(
        merged.elements.contains_key(&Element::InputNumber),
        "v1 should NOT block input-number"
    );
    assert!(
        merged.elements.contains_key(&Element::Level),
        "Child level should be present"
    );
    assert!(
        merged.elements.contains_key(&Element::Input),
        "Child input should be present"
    );

    // In v1, level sections merge (not replaced)
    let error_level = &merged.levels[&Level::Error];
    assert!(
        error_level.contains_key(&Element::Message),
        "v1 should preserve base error message"
    );
    assert!(
        error_level.contains_key(&Element::Time),
        "v1 should have child error time"
    );
}

#[test]
fn test_v1_level_overrides_with_styles() {
    // FR-021a: V1 level overrides MUST support v1 features like style references
    // This test verifies that level-specific overrides can use style definitions
    // Uses external file: src/testing/assets/themes/v1-level-with-styles.yaml

    let theme = raw_theme("v1-level-with-styles");

    // Verify it's a v1 theme
    assert_eq!(theme.version, Version::V1_0);

    // Verify the theme has styles defined
    assert!(!theme.styles.is_empty(), "V1 theme should have style definitions");

    // Verify base elements
    assert_eq!(
        theme.elements[&Element::Message].foreground,
        Some(Color::RGB(RGB(255, 255, 255))),
        "Base message should be white"
    );

    // Verify level-specific overrides exist
    let error_level = Level::Error;
    assert!(
        theme.levels.contains_key(&error_level),
        "Theme should have error level overrides"
    );

    // Verify level overrides reference styles (v1 feature)
    let error_message = theme
        .levels
        .get(&error_level)
        .and_then(|pack| pack.get(&Element::Message));
    assert!(error_message.is_some(), "Error level should override message element");

    // The style reference should be preserved in the base
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
    // FR-029: System MUST report file format parse errors with helpful messages
    // This test verifies that malformed theme files produce clear error messages
    // Uses external files: src/testing/assets/themes/malformed.{yaml,toml,json}

    // Test YAML parse error
    let yaml_result = Theme::load(&dirs(), "malformed.yaml");
    assert!(yaml_result.is_err(), "Malformed YAML should produce an error");
    let yaml_err = yaml_result.unwrap_err();
    let yaml_msg = yaml_err.to_string();
    // Error message should mention it's a YAML error or parsing issue
    assert!(
        yaml_msg.contains("malformed.yaml") || yaml_msg.contains("YAML") || yaml_msg.contains("parse"),
        "YAML error should be descriptive, got: {}",
        yaml_msg
    );

    // Test TOML parse error
    let toml_result = Theme::load(&dirs(), "malformed.toml");
    assert!(toml_result.is_err(), "Malformed TOML should produce an error");
    let toml_err = toml_result.unwrap_err();
    let toml_msg = toml_err.to_string();
    // Error message should mention it's a TOML error or parsing issue
    assert!(
        toml_msg.contains("malformed.toml") || toml_msg.contains("TOML") || toml_msg.contains("parse"),
        "TOML error should be descriptive, got: {}",
        toml_msg
    );

    // Test JSON parse error
    let json_result = Theme::load(&dirs(), "malformed.json");
    assert!(json_result.is_err(), "Malformed JSON should produce an error");
    let json_err = json_result.unwrap_err();
    let json_msg = json_err.to_string();
    // Error message should mention it's a JSON error or parsing issue
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
fn test_style_from_role() {
    let style = RawStyle::from(Role::Primary);
    assert!(!style.base.is_empty());
    assert_eq!(style.base.len(), 1);
    assert_eq!(style.base[0], Role::Primary);
}

#[test]
fn test_style_from_vec_roles() {
    let style = RawStyle::from(vec![Role::Primary, Role::Secondary]);
    assert!(!style.base.is_empty());
    assert_eq!(style.base.len(), 2);
    assert_eq!(style.base[0], Role::Primary);
    assert_eq!(style.base[1], Role::Secondary);
}

#[test]
fn test_resolved_style_builder_methods() {
    let style = RawStyle::default()
        .modes(Mode::Bold)
        .foreground(Some(Color::Plain(PlainColor::Red)))
        .background(Some(Color::Plain(PlainColor::Blue)));

    assert_eq!(style.modes, Mode::Bold.into());
    assert_eq!(style.foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(style.background, Some(Color::Plain(PlainColor::Blue)));
}

#[test]
fn test_indicator_pack_merge() {
    let mut base = v1::IndicatorPack::<RawStyle>::default();
    let mut other = v1::IndicatorPack::<RawStyle>::default();

    other.sync.synced.text = "✓".to_string();
    other.sync.failed.text = "✗".to_string();

    base.merge(other, MergeFlags::default());
    assert_eq!(base.sync.synced.text, "✓");
    assert_eq!(base.sync.failed.text, "✗");
}

#[test]
fn test_indicator_style_merge_empty() {
    let mut base = v1::IndicatorStyle::<RawStyle>::default();
    let other = v1::IndicatorStyle::<RawStyle> {
        prefix: "[".to_string(),
        suffix: "]".to_string(),
        ..Default::default()
    };

    base.merge(other, MergeFlags::default());
    assert_eq!(base.prefix, "[");
    assert_eq!(base.suffix, "]");
}

#[test]
fn test_serde_display_success() {
    use crate::themecfg::Role;
    let wrapper = display(&Role::Primary);
    let display_str = format!("{}", wrapper);
    assert!(display_str.contains("primary"));
}

#[test]
fn test_resolved_style_merged_style_additive() {
    let base = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Bold.into(),
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: None,
    };

    let patch = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Italic.into(),
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: Some(Color::Plain(PlainColor::Blue)),
    };

    let merged = base.merged(&patch, MergeFlags::default());
    assert_eq!(merged.modes, ModeSetDiff::from(Mode::Bold | Mode::Italic));
    assert_eq!(merged.foreground, Some(Color::Plain(PlainColor::Green)));
    assert_eq!(merged.background, Some(Color::Plain(PlainColor::Blue)));
}

#[test]
fn test_child_blocking_parent_in_style_pack() {
    let mut base = v1::StylePack::default();
    base.insert(Element::Level, RawStyle::default());

    let mut patch = v1::StylePack::default();
    patch.insert(Element::LevelInner, RawStyle::default());

    let merged = base.merged(&patch, Version::V0.merge_flags());

    assert!(!merged.contains_key(&Element::Level));
    assert!(merged.contains_key(&Element::LevelInner));
}

#[test]
fn test_resolved_style_merged_style_replace_modes() {
    let base = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Bold.into(),
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: None,
    };

    let patch = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Italic.into(),
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: None,
    };

    let merged = base.merged(&patch, Version::V0.merge_flags());
    assert_eq!(merged.modes, Mode::Italic.into());
    assert_eq!(merged.foreground, Some(Color::Plain(PlainColor::Green)));
}

#[test]
fn test_sync_indicator_pack_merge() {
    let mut base = v1::SyncIndicatorPack::<RawStyle>::default();
    let mut other = v1::SyncIndicatorPack::<RawStyle>::default();

    other.synced.text = "✓".to_string();
    other.failed.text = "✗".to_string();

    base.merge(other, MergeFlags::default());
    assert_eq!(base.synced.text, "✓");
    assert_eq!(base.failed.text, "✗");
}

#[test]
fn test_indicator_merge_empty_text() {
    let mut base = v1::Indicator::<RawStyle> {
        text: "original".to_string(),
        ..Default::default()
    };

    let other = v1::Indicator::<RawStyle> {
        text: "".to_string(),
        ..Default::default()
    };

    base.merge(other, MergeFlags::default());
    assert_eq!(base.text, "original");
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
fn test_indicator_pack_merged() {
    let base = v1::IndicatorPack::<RawStyle>::default();
    let mut other = v1::IndicatorPack::<RawStyle>::default();
    other.sync.synced.text = "✓".to_string();

    let merged = base.merged(other, MergeFlags::default());
    assert_eq!(merged.sync.synced.text, "✓");
}

#[test]
fn test_sync_indicator_pack_merged() {
    let base = v1::SyncIndicatorPack::<RawStyle>::default();
    let mut other = v1::SyncIndicatorPack::<RawStyle>::default();
    other.synced.text = "✓".to_string();

    let merged = base.merged(other, MergeFlags::default());
    assert_eq!(merged.synced.text, "✓");
}

#[test]
fn test_indicator_text_merge() {
    let base = v1::Indicator::<RawStyle>::default();
    let other = v1::Indicator::<RawStyle> {
        text: "test".to_string(),
        ..Default::default()
    };

    let merged = base.merged(other, MergeFlags::default());
    assert_eq!(merged.text, "test");
}

#[test]
fn test_indicator_style_defaults() {
    let style = v1::IndicatorStyle::<RawStyle>::default();
    let other = v1::IndicatorStyle::<RawStyle> {
        prefix: "[".to_string(),
        suffix: "]".to_string(),
        ..Default::default()
    };

    let merged = style.merged(other, MergeFlags::default());
    assert_eq!(merged.prefix, "[");
    assert_eq!(merged.suffix, "]");
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
    // Test that v1 themes strictly reject unknown top-level keys (fail-fast)
    // This is different from v0 which silently ignores unknown keys for forward compatibility
    let result = load_raw_theme_unmerged("v1-unknown-key");

    // v1 should fail on unknown keys due to #[serde(deny_unknown_fields)]
    assert!(
        result.is_err(),
        "v1 theme with unknown key should fail strict validation"
    );

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // The error message should mention the unknown field
    assert!(
        err_msg.contains("unknown") || err_msg.contains("field"),
        "Error message should indicate unknown field, got: {}",
        err_msg
    );
}

#[test]
fn test_v1_strict_unknown_enum_variant_rejected() {
    // Test that v1 themes strictly reject unknown enum variants (fail-fast)
    // This tests unknown Role variant in the styles section
    let result = load_raw_theme_unmerged("v1-unknown-role");

    // v1 should fail on unknown enum variants
    assert!(
        result.is_err(),
        "v1 theme with unknown Role variant should fail strict validation"
    );

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // The error message should mention the unknown variant or role
    assert!(
        err_msg.contains("unknown") || err_msg.contains("variant") || err_msg.contains("future-role"),
        "Error message should indicate unknown enum variant, got: {}",
        err_msg
    );
}

#[test]
fn test_v1_schema_field_accepted() {
    // Test that v1 themes can include $schema field for IDE/validator support
    // The field should be accepted and ignored during processing
    let result = load_raw_theme_unmerged("v1-with-schema");

    assert!(
        result.is_ok(),
        "v1 theme with $schema field should be accepted, got error: {:?}",
        result.err()
    );

    // Verify the theme loads and resolves correctly
    let theme = result.unwrap();
    let resolved = theme.resolve();
    assert!(resolved.is_ok(), "Theme with $schema should resolve successfully");

    let resolved = resolved.unwrap();
    // After resolution, LevelInner is added via parent→inner inheritance (Level → LevelInner)
    assert_eq!(resolved.elements.len(), 3, "Should have 3 elements after resolution");
}

#[test]
fn test_style_base_display_multiple_roles() {
    let base = v1::StyleBase::from(vec![Role::Primary, Role::Secondary]);
    let s = format!("{}", base);
    assert!(s.contains(","));
}

#[test]
fn test_style_pack_merged() {
    let mut items1 = HashMap::new();
    items1.insert(Role::Primary, v1::Style::default());
    let pack1 = v1::StylePack::<Role, v1::Style>::new(items1);

    let mut items2 = HashMap::new();
    let style2 = v1::Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        ..v1::Style::default()
    };
    items2.insert(Role::Secondary, style2);
    let pack2 = v1::StylePack::<Role, v1::Style>::new(items2);

    let merged = pack1.merged(pack2);
    let mut resolver = v1::StyleResolver::new(&merged, MergeFlags::default());
    assert!(resolver.resolve(&Role::Primary).is_ok());
    assert!(resolver.resolve(&Role::Secondary).is_ok());
}

#[test]
fn test_v1_style_reverse_merge() {
    let mut style1 = v1::Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        ..v1::Style::default()
    };

    let mut style2 = v1::Style {
        foreground: Some(Color::Plain(PlainColor::Blue)),
        ..v1::Style::default()
    };
    style2.modes.adds.insert(Mode::Bold);

    style1.reverse_merge(style2, MergeFlags::default());
    assert_eq!(style1.foreground, Some(Color::Plain(PlainColor::Red)));
    assert!(style1.modes.adds.contains(Mode::Bold));
}

#[test]
fn test_v1_style_resolve_base_with() {
    let bases = v1::StyleBase::from(vec![Role::Primary]);
    let style = v1::Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        ..v1::Style::default()
    };

    let resolved = v1::Style::resolve_base_with(&bases, &style, MergeFlags::default(), |_| {
        let mut rs = Style::new();
        rs.modes.adds.insert(Mode::Bold);
        rs
    });

    assert_eq!(resolved.foreground, Some(Color::Plain(PlainColor::Red)));
    assert!(resolved.modes.adds.contains(Mode::Bold));
}

#[test]
fn test_v1_style_resolve_with() {
    let bases = v1::StyleBase::from(vec![Role::Primary]);
    let style = v1::Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        ..v1::Style::default()
    };

    let resolved = v1::Style::resolve_with(&bases, &style, MergeFlags::default(), |_| {
        let mut rs = Style::new();
        rs.modes.adds.insert(Mode::Bold);
        rs
    });

    assert_eq!(resolved.foreground, Some(Color::Plain(PlainColor::Red)));
    assert!(resolved.modes.adds.contains(Mode::Bold));
}

#[test]
fn test_v1_style_merge_owned() {
    let mut style1 = v1::Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        base: v1::StyleBase::from(vec![Role::Primary]),
        ..v1::Style::default()
    };
    style1.modes.adds.insert(Mode::Bold);

    let mut style2 = v1::Style {
        foreground: Some(Color::Plain(PlainColor::Blue)),
        base: v1::StyleBase::from(vec![Role::Secondary]),
        ..v1::Style::default()
    };
    style2.modes.adds.insert(Mode::Italic);

    style1.merge(style2, MergeFlags::default());
    assert_eq!(style1.foreground, Some(Color::Plain(PlainColor::Blue)));
    assert!(style1.modes.adds.contains(Mode::Bold));
    assert!(style1.modes.adds.contains(Mode::Italic));
    assert_eq!(style1.base, v1::StyleBase::from(vec![Role::Secondary]));
}

#[test]
fn test_v0_theme_merge_flags() {
    let theme: v0::Theme = load_yaml_fixture("fixtures/themes/v0-theme-merge-flags.yaml");
    let flags = theme.merge_flags();
    assert!(flags.contains(MergeFlag::ReplaceElements));
    assert!(flags.contains(MergeFlag::ReplaceHierarchies));
    assert!(flags.contains(MergeFlag::ReplaceModes));
}

#[test]
fn test_v0_style_new() {
    let style = v0::Style::new();
    assert!(style.modes.is_empty());
    assert_eq!(style.foreground, None);
    assert_eq!(style.background, None);
}

#[test]
fn test_v0_style_default_ref() {
    let style: &v0::Style = Default::default();
    assert!(style.modes.is_empty());
    assert_eq!(style.foreground, None);
    assert_eq!(style.background, None);
}

#[test]
fn test_v0_style_pack_from_hashmap() {
    let mut map = HashMap::new();
    map.insert(Element::Message, v0::Style::new());
    let pack = v0::StylePack::from(map);
    assert_eq!(pack.len(), 1);
}

#[test]
fn test_v0_style_pack_deserialize() {
    let pack: v0::StylePack = load_yaml_fixture("style-packs/v0-pack.yaml");
    assert!(pack.contains_key(&Element::Message));
}
