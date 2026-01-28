//! Pager selection logic.
//!
//! This module provides the `PagerSelector` which handles pager selection
//! based on environment variables, configuration, and executable availability.

use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::path::Path;

use super::config::{PagerConfig, PagerProfile, PagerRole};

// ---

/// Represents a resolved pager specification from environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PagerSpec {
    /// References a named profile.
    Profile(String),
    /// Direct command (parsed from env var).
    Command(Vec<String>),
    /// Explicitly disabled (empty string in env var).
    Disabled,
}

// ---

/// Represents the final pager selection result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedPager {
    /// Use a pager with the given command, args, and env.
    Pager {
        command: Vec<String>,
        env: HashMap<String, String>,
    },
    /// No pager, output to stdout.
    None,
}

// ---

/// Trait for providing environment variable access.
/// Allows dependency injection for testing.
pub trait EnvProvider {
    /// Gets an environment variable value.
    fn get(&self, name: &str) -> Option<String>;
}

/// Default environment provider that reads from the actual environment.
pub struct SystemEnv;

impl EnvProvider for SystemEnv {
    fn get(&self, name: &str) -> Option<String> {
        env::var(name).ok()
    }
}

// ---

/// Trait for checking executable availability.
/// Allows dependency injection for testing.
pub trait ExeChecker {
    /// Checks if an executable is available in PATH.
    fn is_available(&self, executable: &str) -> bool;
}

/// Default executable checker that uses the `which` crate.
pub struct SystemExeChecker;

impl ExeChecker for SystemExeChecker {
    fn is_available(&self, executable: &str) -> bool {
        which::which(executable).is_ok()
    }
}

// ---

/// Pager selector that handles pager selection based on environment variables,
/// configuration, and executable availability.
pub struct PagerSelector<'a, E = SystemEnv, C = SystemExeChecker> {
    /// The pager configuration (profile priority list).
    config: Option<&'a PagerConfig>,
    /// Available pager profiles.
    profiles: &'a HashMap<String, PagerProfile>,
    /// Environment provider.
    env_provider: E,
    /// Executable checker.
    exe_checker: C,
}

impl<'a> PagerSelector<'a, SystemEnv, SystemExeChecker> {
    /// Creates a new pager selector with the given configuration.
    pub fn new(config: Option<&'a PagerConfig>, profiles: &'a HashMap<String, PagerProfile>) -> Self {
        Self {
            config,
            profiles,
            env_provider: SystemEnv,
            exe_checker: SystemExeChecker,
        }
    }
}

impl<'a, E: EnvProvider, C: ExeChecker> PagerSelector<'a, E, C> {
    /// Creates a new pager selector with custom environment and executable checker.
    pub fn with_providers(
        config: Option<&'a PagerConfig>,
        profiles: &'a HashMap<String, PagerProfile>,
        env_provider: E,
        exe_checker: C,
    ) -> Self {
        Self {
            config,
            profiles,
            env_provider,
            exe_checker,
        }
    }

    /// Selects a pager for the given role.
    pub fn select(&self, role: PagerRole) -> SelectedPager {
        match role {
            PagerRole::View => self.select_for_view(),
            PagerRole::Follow => self.select_for_follow(),
        }
    }

    /// Selects a pager for view mode.
    ///
    /// Precedence order:
    /// 1. `HL_PAGER` env var (profile name or command string; empty = disabled)
    /// 2. Config `pager` setting (priority list of profiles)
    /// 3. `PAGER` env var (backward compatibility)
    /// 4. Fall back to stdout
    fn select_for_view(&self) -> SelectedPager {
        // 1. Check HL_PAGER env var
        if let Some(spec) = self.resolve_env_var("HL_PAGER") {
            match spec {
                PagerSpec::Disabled => return SelectedPager::None,
                PagerSpec::Profile(name) => {
                    // Try as profile first, then fall back to treating as command
                    if let Some(selected) = self.try_profile(&name, PagerRole::View) {
                        return selected;
                    }
                    // Profile not found, try as a direct command
                    if let Some(selected) = self.try_command(vec![name]) {
                        return selected;
                    }
                    // Command not available, fall through
                }
                PagerSpec::Command(cmd) => {
                    if let Some(selected) = self.try_command(cmd) {
                        return selected;
                    }
                    // Command not available, fall through
                }
            }
        }

        // 2. Check config pager setting
        if let Some(config) = self.config {
            for profile_name in config.profiles() {
                if let Some(selected) = self.try_profile(profile_name, PagerRole::View) {
                    return selected;
                }
            }
        }

        // 3. Check PAGER env var (backward compatibility)
        if let Some(spec) = self.resolve_env_var("PAGER") {
            match spec {
                PagerSpec::Disabled => return SelectedPager::None,
                PagerSpec::Profile(name) => {
                    // Try as profile first, then fall back to treating as command
                    if let Some(selected) = self.try_profile(&name, PagerRole::View) {
                        return selected;
                    }
                    // Profile not found, try as a direct command
                    if let Some(selected) = self.try_command(vec![name]) {
                        return selected;
                    }
                }
                PagerSpec::Command(cmd) => {
                    if let Some(selected) = self.try_command(cmd) {
                        return selected;
                    }
                }
            }
        }

        // 4. Fall back to stdout
        SelectedPager::None
    }

    /// Selects a pager for follow mode.
    ///
    /// Precedence order:
    /// 1. `HL_FOLLOW_PAGER` env var (profile name or command string; empty = disabled)
    /// 2. `HL_PAGER` env var (unless empty, which is overridden by HL_FOLLOW_PAGER)
    /// 3. Config `pager` setting (only if profile has `follow.enabled = true`)
    /// 4. Fall back to stdout
    fn select_for_follow(&self) -> SelectedPager {
        // 1. Check HL_FOLLOW_PAGER env var (takes precedence)
        if let Some(spec) = self.resolve_env_var("HL_FOLLOW_PAGER") {
            match spec {
                PagerSpec::Disabled => return SelectedPager::None,
                PagerSpec::Profile(name) => {
                    // Try as profile first, then fall back to treating as command
                    if let Some(selected) = self.try_profile(&name, PagerRole::Follow) {
                        return selected;
                    }
                    // Profile not found, try as a direct command
                    if let Some(selected) = self.try_command(vec![name]) {
                        return selected;
                    }
                }
                PagerSpec::Command(cmd) => {
                    if let Some(selected) = self.try_command(cmd) {
                        return selected;
                    }
                }
            }
        }

        // 2. Check HL_PAGER env var
        if let Some(spec) = self.resolve_env_var("HL_PAGER") {
            match spec {
                PagerSpec::Disabled => {
                    // HL_PAGER="" disables pager, but HL_FOLLOW_PAGER can override
                    // (already checked above), so return None here
                    return SelectedPager::None;
                }
                PagerSpec::Profile(name) => {
                    // Only use if profile has follow enabled
                    if let Some(profile) = self.profiles.get(&name) {
                        if profile.follow.is_enabled(PagerRole::Follow) {
                            if let Some(selected) = self.try_profile(&name, PagerRole::Follow) {
                                return selected;
                            }
                        }
                    } else {
                        // Profile not found, try as a direct command for follow mode
                        if let Some(selected) = self.try_command(vec![name]) {
                            return selected;
                        }
                    }
                }
                PagerSpec::Command(cmd) => {
                    // Direct command from env var - use it for follow mode
                    if let Some(selected) = self.try_command(cmd) {
                        return selected;
                    }
                }
            }
        }

        // 3. Check config pager setting (only if profile has follow.enabled = true)
        if let Some(config) = self.config {
            for profile_name in config.profiles() {
                if let Some(profile) = self.profiles.get(profile_name) {
                    if profile.follow.is_enabled(PagerRole::Follow) {
                        if let Some(selected) = self.try_profile(profile_name, PagerRole::Follow) {
                            return selected;
                        }
                    }
                }
            }
        }

        // 4. Fall back to stdout
        SelectedPager::None
    }

    /// Resolves an environment variable to a `PagerSpec`.
    ///
    /// - Empty string → `PagerSpec::Disabled`
    /// - Matches a simple name (no spaces, no path separators) → `PagerSpec::Profile`
    /// - Otherwise → `PagerSpec::Command` (parsed with shellwords)
    fn resolve_env_var(&self, name: &str) -> Option<PagerSpec> {
        let value = self.env_provider.get(name)?;

        if value.is_empty() {
            return Some(PagerSpec::Disabled);
        }

        // Check if it looks like a profile name (simple identifier, no spaces or path separators)
        if is_profile_name(&value) {
            return Some(PagerSpec::Profile(value));
        }

        // Parse as a command string
        match shellwords::split(&value) {
            Ok(parts) if !parts.is_empty() => Some(PagerSpec::Command(parts)),
            _ => Some(PagerSpec::Command(vec![value])),
        }
    }

    /// Tries to use a named profile, returning `Some(SelectedPager)` if successful.
    fn try_profile(&self, name: &str, role: PagerRole) -> Option<SelectedPager> {
        let profile = self.profiles.get(name)?;

        if !profile.is_valid() {
            log::debug!("pager profile '{}' has no command, skipping", name);
            return None;
        }

        let executable = profile.executable()?;
        if !self.exe_checker.is_available(executable) {
            log::debug!("pager '{}' not found in PATH, skipping profile '{}'", executable, name);
            return None;
        }

        let command = profile.build_command(role).into_iter().map(String::from).collect();
        let env = profile.env.clone();

        log::debug!("selected pager profile '{}' for {:?} mode", name, role);

        Some(SelectedPager::Pager { command, env })
    }

    /// Tries to use a direct command, returning `Some(SelectedPager)` if successful.
    fn try_command(&self, command: Vec<String>) -> Option<SelectedPager> {
        let executable = command.first()?;
        if !self.exe_checker.is_available(executable) {
            log::debug!("pager '{}' not found in PATH", executable);
            return None;
        }

        // Apply special handling for `less`
        let (command, env) = apply_less_defaults(command);

        log::debug!("selected pager command: {:?}", command);

        Some(SelectedPager::Pager { command, env })
    }
}

// ---

/// Checks if a string looks like a profile name (simple identifier).
fn is_profile_name(s: &str) -> bool {
    !s.is_empty() && !s.contains(char::is_whitespace) && !s.contains('/') && !s.contains('\\') && !s.contains('=')
}

/// Applies default settings for `less` command.
///
/// When using `less` via direct command (not profile), we auto-add `-R` for ANSI colors
/// and set `LESSCHARSET=UTF-8`.
fn apply_less_defaults(mut command: Vec<String>) -> (Vec<String>, HashMap<String, String>) {
    let mut env = HashMap::new();

    if let Some(executable) = command.first() {
        let exe_name = Path::new(executable)
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or(executable);

        if exe_name == "less" {
            // Add -R if not already present
            if !command
                .iter()
                .any(|arg| arg == "-R" || arg.starts_with("-R") || arg.contains('R'))
            {
                command.push("-R".to_string());
            }
            // Set LESSCHARSET
            env.insert("LESSCHARSET".to_string(), "UTF-8".to_string());
        }
    }

    (command, env)
}

// ---

/// Checks if an executable is available in PATH.
pub fn is_available(executable: &str) -> bool {
    which::which(executable).is_ok()
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_profile_name_simple() {
        assert!(is_profile_name("less"));
        assert!(is_profile_name("fzf"));
        assert!(is_profile_name("my-pager"));
        assert!(is_profile_name("pager_v2"));
    }

    #[test]
    fn is_profile_name_rejects_commands() {
        assert!(!is_profile_name("less -R"));
        assert!(!is_profile_name("/usr/bin/less"));
        assert!(!is_profile_name("FOO=bar less"));
        assert!(!is_profile_name(""));
    }

    #[test]
    fn apply_less_defaults_adds_flag() {
        let (cmd, env) = apply_less_defaults(vec!["less".to_string()]);
        assert!(cmd.contains(&"-R".to_string()));
        assert_eq!(env.get("LESSCHARSET"), Some(&"UTF-8".to_string()));
    }

    #[test]
    fn apply_less_defaults_preserves_existing_flag() {
        let (cmd, _) = apply_less_defaults(vec!["less".to_string(), "-R".to_string()]);
        assert_eq!(cmd.iter().filter(|&a| a == "-R").count(), 1);
    }

    #[test]
    fn apply_less_defaults_ignores_other_pagers() {
        let (cmd, env) = apply_less_defaults(vec!["fzf".to_string(), "--ansi".to_string()]);
        assert_eq!(cmd, vec!["fzf", "--ansi"]);
        assert!(env.is_empty());
    }
}
