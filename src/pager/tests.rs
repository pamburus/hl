//! Tests for the pager module.

use std::collections::HashMap;

use super::config::{PagerConfig, PagerProfile, PagerRole, PagerRoleConfig};
use super::selection::{EnvProvider, ExeChecker, PagerSelector, SelectedPager};
use crate::output::OutputDelimiter;

// ---
// Test data files embedded at compile time
// ---

const SINGLE_PROFILE: &str = include_str!("../testing/assets/pagers/single-profile.toml");
const PROFILE_WITH_ENV: &str = include_str!("../testing/assets/pagers/profile-with-env.toml");
const PRIORITY_LIST: &str = include_str!("../testing/assets/pagers/priority-list.toml");
const FOLLOW_ENABLED: &str = include_str!("../testing/assets/pagers/follow-enabled.toml");
const MINIMAL_PROFILE: &str = include_str!("../testing/assets/pagers/minimal-profile.toml");
const PROFILE_WITH_VIEW_ARGS: &str = include_str!("../testing/assets/pagers/profile-with-view-args.toml");
const EMPTY_PRIORITY: &str = include_str!("../testing/assets/pagers/empty-priority.toml");
const UNAVAILABLE_FIRST: &str = include_str!("../testing/assets/pagers/unavailable-first.toml");

// ---
// PagerConfig deserialization tests
// ---

#[test]
fn pager_config_single_profile() {
    let config: TestConfig = toml::from_str(SINGLE_PROFILE).expect("failed to parse");

    assert_eq!(config.pager, Some(PagerConfig::Single("less".to_string())));
    assert_eq!(
        config.pager.as_ref().unwrap().profiles().collect::<Vec<_>>(),
        vec!["less"]
    );
}

#[test]
fn pager_config_priority_list() {
    let config: TestConfig = toml::from_str(PRIORITY_LIST).expect("failed to parse");

    assert_eq!(
        config.pager,
        Some(PagerConfig::Priority(vec!["fzf".to_string(), "less".to_string()]))
    );
    assert_eq!(
        config.pager.as_ref().unwrap().profiles().collect::<Vec<_>>(),
        vec!["fzf", "less"]
    );
}

#[test]
fn pager_config_empty_priority_list() {
    let config: TestConfig = toml::from_str(EMPTY_PRIORITY).expect("failed to parse");

    assert_eq!(config.pager, Some(PagerConfig::Priority(vec![])));
    assert!(config.pager.as_ref().unwrap().profiles().next().is_none());
}

// ---
// PagerProfile deserialization tests
// ---

#[test]
fn pager_profile_minimal() {
    let config: TestConfig = toml::from_str(MINIMAL_PROFILE).expect("failed to parse");

    let profile = &config.pagers["less"];
    assert_eq!(profile.command, vec!["less", "-R"]);
    assert!(profile.env.is_empty());
    assert!(profile.view.args.is_empty());
    assert!(profile.follow.args.is_empty());
    assert_eq!(profile.follow.enabled, None);
}

#[test]
fn pager_profile_with_env() {
    let config: TestConfig = toml::from_str(PROFILE_WITH_ENV).expect("failed to parse");

    let profile = &config.pagers["less"];
    assert_eq!(&profile.env["LESSCHARSET"], "UTF-8");
}

#[test]
fn pager_profile_with_view_args() {
    let config: TestConfig = toml::from_str(PROFILE_WITH_VIEW_ARGS).expect("failed to parse");

    let profile = &config.pagers["fzf"];
    assert_eq!(profile.view.args, vec!["--layout=reverse-list"]);
}

#[test]
fn pager_profile_with_follow_enabled() {
    let config: TestConfig = toml::from_str(FOLLOW_ENABLED).expect("failed to parse");

    let profile = &config.pagers["fzf"];
    assert_eq!(profile.follow.enabled, Some(true));
    assert_eq!(profile.follow.args, vec!["--tac", "--track"]);
}

#[test]
fn pager_profile_follow_disabled_by_default() {
    let config: TestConfig = toml::from_str(MINIMAL_PROFILE).expect("failed to parse");

    let profile = &config.pagers["less"];
    // follow.enabled is None by default, which means disabled
    assert!(!profile.follow.is_enabled(PagerRole::Follow));
}

#[test]
fn pager_profile_view_always_enabled() {
    let config: TestConfig = toml::from_str(MINIMAL_PROFILE).expect("failed to parse");

    let profile = &config.pagers["less"];
    // view is always enabled
    assert!(profile.view.is_enabled(PagerRole::View));
}

// ---
// PagerProfile method tests
// ---

#[test]
fn pager_profile_executable() {
    let profile = profile_with_command(vec!["less", "-R"]);
    assert_eq!(profile.executable(), Some("less"));

    let empty = profile_with_command(vec![]);
    assert_eq!(empty.executable(), None);
}

#[test]
fn pager_profile_build_command_view() {
    let config: TestConfig = toml::from_str(FOLLOW_ENABLED).expect("failed to parse");

    let profile = &config.pagers["fzf"];
    let cmd = profile.build_command(PagerRole::View);
    assert_eq!(cmd, vec!["fzf", "--ansi", "--layout=reverse-list"]);
}

#[test]
fn pager_profile_build_command_follow() {
    let config: TestConfig = toml::from_str(FOLLOW_ENABLED).expect("failed to parse");

    let profile = &config.pagers["fzf"];
    let cmd = profile.build_command(PagerRole::Follow);
    assert_eq!(cmd, vec!["fzf", "--ansi", "--tac", "--track"]);
}

// ---
// Helper types and functions
// ---

#[derive(Debug, serde::Deserialize)]
struct TestConfig {
    #[serde(default)]
    pager: Option<PagerConfig>,
    #[serde(default)]
    pagers: HashMap<String, PagerProfile>,
}

fn profile_with_command(command: Vec<&str>) -> PagerProfile {
    PagerProfile {
        command: command.into_iter().map(String::from).collect(),
        env: HashMap::new(),
        delimiter: None,
        view: PagerRoleConfig::default(),
        follow: PagerRoleConfig::default(),
    }
}

// ---
// Mock providers for testing
// ---

/// Mock environment provider for isolated testing.
struct MockEnv {
    vars: HashMap<String, String>,
}

impl MockEnv {
    fn new() -> Self {
        Self { vars: HashMap::new() }
    }

    fn with_var(mut self, name: &str, value: &str) -> Self {
        self.vars.insert(name.to_string(), value.to_string());
        self
    }
}

impl EnvProvider for MockEnv {
    fn get(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
}

/// Mock executable checker for isolated testing.
struct MockExeChecker {
    available: Vec<String>,
}

impl MockExeChecker {
    fn with_available(executables: &[&str]) -> Self {
        Self {
            available: executables.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl ExeChecker for MockExeChecker {
    fn is_available(&self, executable: &str) -> bool {
        self.available.contains(&executable.to_string())
    }
}

fn selector_with_mocks<'a>(
    config: Option<&'a PagerConfig>,
    profiles: &'a HashMap<String, PagerProfile>,
    env: MockEnv,
    available: &[&str],
) -> PagerSelector<'a, MockEnv, MockExeChecker> {
    PagerSelector::with_providers(config, profiles, env, MockExeChecker::with_available(available))
}

// ---
// PagerSelector tests
// ---

#[test]
fn selector_view_with_single_available_profile() {
    let config: TestConfig = toml::from_str(SINGLE_PROFILE).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let selector = selector_with_mocks(pager_config, &config.pagers, MockEnv::new(), &["less"]);

    let selected = selector.select(PagerRole::View);

    assert!(matches!(selected, SelectedPager::Pager { .. }));
    if let SelectedPager::Pager { command, env, .. } = selected {
        assert_eq!(command[0], "less");
        assert!(command.contains(&"-R".to_string()));
        assert!(command.contains(&"--mouse".to_string()));
        assert_eq!(env.get("LESSCHARSET"), Some(&"UTF-8".to_string()));
    }
}

#[test]
fn selector_view_with_priority_fallback() {
    let config: TestConfig = toml::from_str(UNAVAILABLE_FIRST).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    // Only `less` is available, not the nonexistent pager
    let selector = selector_with_mocks(pager_config, &config.pagers, MockEnv::new(), &["less"]);

    let selected = selector.select(PagerRole::View);

    assert!(matches!(selected, SelectedPager::Pager { .. }));
    if let SelectedPager::Pager { command, .. } = selected {
        assert_eq!(command[0], "less");
    }
}

#[test]
fn selector_view_with_empty_priority_returns_none() {
    let config: TestConfig = toml::from_str(EMPTY_PRIORITY).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let selector = selector_with_mocks(pager_config, &config.pagers, MockEnv::new(), &["less"]);

    let selected = selector.select(PagerRole::View);
    assert!(matches!(selected, SelectedPager::None));
}

#[test]
fn selector_follow_disabled_by_default() {
    let config: TestConfig = toml::from_str(MINIMAL_PROFILE).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let selector = selector_with_mocks(pager_config, &config.pagers, MockEnv::new(), &["less"]);

    let selected = selector.select(PagerRole::Follow);

    // Follow mode should return None since follow.enabled is not set
    assert!(matches!(selected, SelectedPager::None));
}

#[test]
fn selector_follow_when_enabled() {
    let config: TestConfig = toml::from_str(FOLLOW_ENABLED).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let selector = selector_with_mocks(pager_config, &config.pagers, MockEnv::new(), &["fzf"]);

    let selected = selector.select(PagerRole::Follow);

    assert!(matches!(selected, SelectedPager::Pager { .. }));
    if let SelectedPager::Pager { command, .. } = selected {
        assert_eq!(command[0], "fzf");
        // Should include follow.args
        assert!(command.contains(&"--tac".to_string()));
        assert!(command.contains(&"--track".to_string()));
    }
}

#[test]
fn selector_view_env_override_with_profile_using_at_prefix() {
    let config: TestConfig = toml::from_str(PRIORITY_LIST).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let env = MockEnv::new().with_var("HL_PAGER", "@less");
    let selector = selector_with_mocks(pager_config, &config.pagers, env, &["fzf", "less"]);

    let selected = selector.select(PagerRole::View);

    // Should use `less` profile from HL_PAGER, not `fzf` from config priority
    assert!(matches!(selected, SelectedPager::Pager { .. }));
    if let SelectedPager::Pager { command, .. } = selected {
        assert_eq!(command[0], "less");
    }
}

#[test]
fn selector_view_env_override_without_at_prefix_uses_command() {
    let config: TestConfig = toml::from_str(PRIORITY_LIST).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let env = MockEnv::new().with_var("HL_PAGER", "less");
    let selector = selector_with_mocks(pager_config, &config.pagers, env, &["fzf", "less"]);

    let selected = selector.select(PagerRole::View);

    // Without @ prefix, should use `less` as direct command (with -R added automatically)
    assert!(matches!(selected, SelectedPager::Pager { .. }));
    if let SelectedPager::Pager { command, .. } = selected {
        assert_eq!(command[0], "less");
        // Should have -R added by apply_less_defaults
        assert!(command.contains(&"-R".to_string()));
    }
}

#[test]
fn selector_view_env_override_with_command() {
    let config: TestConfig = toml::from_str(SINGLE_PROFILE).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let env = MockEnv::new().with_var("HL_PAGER", "cat -n");
    let selector = selector_with_mocks(pager_config, &config.pagers, env, &["cat", "less"]);

    let selected = selector.select(PagerRole::View);

    assert!(matches!(selected, SelectedPager::Pager { .. }));
    if let SelectedPager::Pager { command, .. } = selected {
        assert_eq!(command, vec!["cat", "-n"]);
    }
}

#[test]
fn selector_view_env_empty_disables_pager() {
    let config: TestConfig = toml::from_str(SINGLE_PROFILE).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let env = MockEnv::new().with_var("HL_PAGER", "");
    let selector = selector_with_mocks(pager_config, &config.pagers, env, &["less"]);

    let selected = selector.select(PagerRole::View);

    assert!(matches!(selected, SelectedPager::None));
}

#[test]
fn selector_follow_env_override() {
    let config: TestConfig = toml::from_str(MINIMAL_PROFILE).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    // HL_FOLLOW_PAGER can enable pager for follow mode even if profile doesn't have follow.enabled
    let env = MockEnv::new().with_var("HL_FOLLOW_PAGER", "less -R");
    let selector = selector_with_mocks(pager_config, &config.pagers, env, &["less"]);

    let selected = selector.select(PagerRole::Follow);

    assert!(matches!(selected, SelectedPager::Pager { .. }));
    if let SelectedPager::Pager { command, .. } = selected {
        assert_eq!(command[0], "less");
    }
}

#[test]
fn selector_follow_hl_follow_pager_overrides_hl_pager_empty() {
    let config: TestConfig = toml::from_str(MINIMAL_PROFILE).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    // HL_PAGER="" but HL_FOLLOW_PAGER is set - should use HL_FOLLOW_PAGER
    let env = MockEnv::new()
        .with_var("HL_PAGER", "")
        .with_var("HL_FOLLOW_PAGER", "less -R");
    let selector = selector_with_mocks(pager_config, &config.pagers, env, &["less"]);

    let selected = selector.select(PagerRole::Follow);

    assert!(matches!(selected, SelectedPager::Pager { .. }));
}

#[test]
fn selector_view_at_prefix_with_nonexistent_profile_returns_none() {
    let config: TestConfig = toml::from_str(MINIMAL_PROFILE).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let env = MockEnv::new().with_var("HL_PAGER", "@nonexistent");
    let selector = selector_with_mocks(pager_config, &config.pagers, env, &["less"]);

    let selected = selector.select(PagerRole::View);

    // @nonexistent refers to a profile that doesn't exist
    assert!(matches!(selected, SelectedPager::None));
}

#[test]
fn selector_follow_at_prefix_uses_profile() {
    let config: TestConfig = toml::from_str(FOLLOW_ENABLED).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    let env = MockEnv::new().with_var("HL_FOLLOW_PAGER", "@fzf");
    let selector = selector_with_mocks(pager_config, &config.pagers, env, &["fzf"]);

    let selected = selector.select(PagerRole::Follow);

    assert!(matches!(selected, SelectedPager::Pager { .. }));
    if let SelectedPager::Pager { command, .. } = selected {
        assert_eq!(command[0], "fzf");
        // Should include follow.args from profile
        assert!(command.contains(&"--tac".to_string()));
        assert!(command.contains(&"--track".to_string()));
    }
}

#[test]
fn selector_view_pager_env_fallback() {
    let config: TestConfig = toml::from_str(EMPTY_PRIORITY).expect("failed to parse");
    let pager_config = config.pager.as_ref();
    // No HL_PAGER, empty config, but PAGER is set
    let env = MockEnv::new().with_var("PAGER", "more");
    let selector = selector_with_mocks(pager_config, &config.pagers, env, &["more"]);

    let selected = selector.select(PagerRole::View);

    assert!(matches!(selected, SelectedPager::Pager { .. }));
    if let SelectedPager::Pager { command, .. } = selected {
        assert_eq!(command[0], "more");
    }
}

#[test]
fn selector_profile_with_delimiter_nul() {
    let mut profiles = HashMap::new();
    profiles.insert(
        "fzf".to_string(),
        PagerProfile {
            command: vec!["fzf".to_string(), "--ansi".to_string()],
            env: HashMap::new(),
            delimiter: Some(OutputDelimiter::Nul),
            view: PagerRoleConfig::default(),
            follow: PagerRoleConfig::default(),
        },
    );
    let config = PagerConfig::Single("fzf".to_string());
    let selector = selector_with_mocks(Some(&config), &profiles, MockEnv::new(), &["fzf"]);

    let selected = selector.select(PagerRole::View);

    if let SelectedPager::Pager { delimiter, .. } = selected {
        assert_eq!(delimiter, Some(OutputDelimiter::Nul));
    } else {
        panic!("expected SelectedPager::Pager");
    }
}

#[test]
fn selector_env_delimiter_overrides_profile() {
    let mut profiles = HashMap::new();
    profiles.insert(
        "fzf".to_string(),
        PagerProfile {
            command: vec!["fzf".to_string()],
            env: HashMap::new(),
            delimiter: Some(OutputDelimiter::Nul),
            view: PagerRoleConfig::default(),
            follow: PagerRoleConfig::default(),
        },
    );
    let config = PagerConfig::Single("fzf".to_string());
    let env = MockEnv::new().with_var("HL_PAGER_DELIMITER", "newline");
    let selector = selector_with_mocks(Some(&config), &profiles, env, &["fzf"]);

    let selected = selector.select(PagerRole::View);

    if let SelectedPager::Pager { delimiter, .. } = selected {
        assert_eq!(delimiter, Some(OutputDelimiter::Newline));
    } else {
        panic!("expected SelectedPager::Pager");
    }
}

#[test]
fn selector_env_delimiter_nul_overrides_default() {
    let mut profiles = HashMap::new();
    profiles.insert(
        "less".to_string(),
        PagerProfile {
            command: vec!["less".to_string()],
            env: HashMap::new(),
            delimiter: None,
            view: PagerRoleConfig::default(),
            follow: PagerRoleConfig::default(),
        },
    );
    let config = PagerConfig::Single("less".to_string());
    let env = MockEnv::new().with_var("HL_PAGER_DELIMITER", "nul");
    let selector = selector_with_mocks(Some(&config), &profiles, env, &["less"]);

    let selected = selector.select(PagerRole::View);

    if let SelectedPager::Pager { delimiter, .. } = selected {
        assert_eq!(delimiter, Some(OutputDelimiter::Nul));
    } else {
        panic!("expected SelectedPager::Pager");
    }
}
