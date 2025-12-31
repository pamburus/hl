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
fn test_serde_display_success() {
    use crate::themecfg::Role;
    let wrapper = display(&Role::Primary);
    let display_str = format!("{}", wrapper);
    assert!(display_str.contains("primary"));
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
