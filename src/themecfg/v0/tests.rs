use std::collections::HashMap;

use super::super::{Element, GetMergeFlags, MergeFlag, tests::load_yaml_fixture};
use super::{Style, StylePack, Theme};

#[test]
fn test_v0_theme_merge_flags() {
    let theme: Theme = load_yaml_fixture("fixtures/themes/v0-theme-merge-flags.yaml");
    let flags = theme.merge_flags();
    assert!(flags.contains(MergeFlag::ReplaceElements));
    assert!(flags.contains(MergeFlag::ReplaceHierarchies));
    assert!(flags.contains(MergeFlag::ReplaceModes));
}

#[test]
fn test_v0_style_new() {
    let style = Style::new();
    assert!(style.modes.is_empty());
    assert_eq!(style.foreground, None);
    assert_eq!(style.background, None);
}

#[test]
fn test_v0_style_default_ref() {
    let style: &Style = Default::default();
    assert!(style.modes.is_empty());
    assert_eq!(style.foreground, None);
    assert_eq!(style.background, None);
}

#[test]
fn test_v0_style_pack_from_hashmap() {
    let mut map = HashMap::new();
    map.insert(Element::Message, Style::new());
    let pack = StylePack::from(map);
    assert_eq!(pack.len(), 1);
}

#[test]
fn test_v0_style_pack_deserialize() {
    let pack: StylePack = load_yaml_fixture("style-packs/v0-pack.yaml");
    assert!(pack.contains_key(&Element::Message));
}
