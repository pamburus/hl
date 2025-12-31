// std imports
use std::{collections::HashMap, path::PathBuf};

// third-party imports

use yaml_peg::serde as yaml;

// local imports
use crate::appdirs::AppDirs;

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
