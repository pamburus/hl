// std imports
use std::path::PathBuf;

// third-party imports

use yaml::serde as yaml;

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

pub(crate) fn load_raw_theme(name: &str) -> Result<RawTheme> {
    Theme::load_raw(&dirs(), name)
}

pub(crate) fn raw_theme(name: &str) -> RawTheme {
    load_raw_theme(name).unwrap()
}

/// Parses the first item of the YAML fixture at `$path` under `src/testing/assets/fixtures/`,
/// embedded at compile time.
macro_rules! yaml_fixture {
    ($path:literal) => {
        $crate::themecfg::tests::parse_yaml_fixture(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/testing/assets/fixtures/",
            $path
        )))
    };
}
pub(crate) use yaml_fixture;

pub(crate) fn parse_yaml_fixture<T>(content: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    let items: Vec<T> = yaml::from_str(content).unwrap();
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
