//! Pager selection logic.
//!
//! This module provides the `PagerSelector` which handles pager selection
//! based on environment variables, configuration, and executable availability.

use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::path::Path;

use super::config::{PagerConfig, PagerProfile, PagerRole};
use crate::output::OutputDelimiter;

// ---

/// Environment variable for overriding the pager.
const HL_PAGER: &str = "HL_PAGER";

/// Environment variable for overriding the pager in follow mode.
const HL_FOLLOW_PAGER: &str = "HL_FOLLOW_PAGER";

/// Standard environment variable for the pager.
const PAGER: &str = "PAGER";

/// Environment variable for overriding the pager delimiter.
const HL_PAGER_DELIMITER: &str = "HL_PAGER_DELIMITER";

// ---

/// Represents a pager override from an environment variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PagerOverride {
    /// A value that could be a profile name or command (parsed from env var).
    Value(Vec<String>),
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
        delimiter: Option<OutputDelimiter>,
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
        log::debug!("selecting pager for {role:?} mode");
        let result = match role {
            PagerRole::View => self.select_for_view(),
            PagerRole::Follow => self.select_for_follow(),
        };
        match &result {
            SelectedPager::Pager { command, .. } => {
                log::debug!("selected pager: {command:?}");
            }
            SelectedPager::None => {
                log::debug!("no pager selected, using stdout");
            }
        }
        result
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
        if let Some(pager) = self.resolve_env_var(HL_PAGER) {
            match pager {
                PagerOverride::Disabled => {
                    log::debug!("{HL_PAGER} is empty, pager disabled");
                    return SelectedPager::None;
                }
                PagerOverride::Value(cmd) => {
                    if let Some(selected) = self.try_override(&cmd, PagerRole::View, HL_PAGER) {
                        return selected;
                    }
                }
            }
        }

        // 2. Check config pager setting
        if let Some(config) = self.config {
            for profile in config.profiles() {
                if let Some(selected) = self.try_profile(profile, PagerRole::View) {
                    return selected;
                }
            }
        }

        // 3. Check PAGER env var (backward compatibility)
        if let Some(pager) = self.resolve_env_var(PAGER) {
            match pager {
                PagerOverride::Disabled => {
                    log::debug!("{PAGER} is empty, pager disabled");
                    return SelectedPager::None;
                }
                PagerOverride::Value(cmd) => {
                    if let Some(selected) = self.try_override(&cmd, PagerRole::View, PAGER) {
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
        if let Some(pager) = self.resolve_env_var(HL_FOLLOW_PAGER) {
            match pager {
                PagerOverride::Disabled => {
                    log::debug!("{HL_FOLLOW_PAGER} is empty, pager disabled");
                    return SelectedPager::None;
                }
                PagerOverride::Value(cmd) => {
                    if let Some(selected) = self.try_override(&cmd, PagerRole::Follow, HL_FOLLOW_PAGER) {
                        return selected;
                    }
                }
            }
        }

        // 2. Check HL_PAGER env var
        if let Some(pager) = self.resolve_env_var(HL_PAGER) {
            match pager {
                PagerOverride::Disabled => {
                    // HL_PAGER="" disables pager, but HL_FOLLOW_PAGER can override
                    // (already checked above), so return None here
                    log::debug!("{HL_PAGER} is empty, pager disabled");
                    return SelectedPager::None;
                }
                PagerOverride::Value(cmd) => {
                    if let Some(selected) = self.try_override(&cmd, PagerRole::Follow, HL_PAGER) {
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

    /// Resolves an environment variable to a `PagerOverride`.
    ///
    /// - Empty string → `PagerOverride::Disabled`
    /// - Otherwise → `PagerOverride::Value` (parsed with shellwords)
    fn resolve_env_var(&self, name: &str) -> Option<PagerOverride> {
        let value = self.env_provider.get(name)?;

        if value.is_empty() {
            return Some(PagerOverride::Disabled);
        }

        // Parse as a command string
        match shellwords::split(&value) {
            Ok(parts) if !parts.is_empty() => Some(PagerOverride::Value(parts)),
            Ok(_) => {
                log::warn!("{name}: parsed to empty command, treating as disabled");
                Some(PagerOverride::Disabled)
            }
            Err(e) => {
                log::warn!("{name}: failed to parse, {e}, using raw value");
                Some(PagerOverride::Value(vec![value]))
            }
        }
    }

    /// Tries to interpret a command spec as either a profile name or a direct command.
    ///
    /// If the first element matches a profile name, uses that profile.
    /// Otherwise, treats the entire spec as a direct command.
    fn try_override(&self, cmd: &[String], role: PagerRole, source: &str) -> Option<SelectedPager> {
        // If it's a single word and matches a profile name, use the profile
        if cmd.len() == 1 {
            let name = &cmd[0];
            if self.profiles.contains_key(name) {
                return self.try_profile(name, role);
            }
        }

        // Otherwise treat as a direct command
        self.try_command(cmd.to_vec(), source)
    }

    /// Tries to use a named profile, returning `Some(SelectedPager)` if successful.
    fn try_profile(&self, name: &str, role: PagerRole) -> Option<SelectedPager> {
        let profile = self.profiles.get(name)?;

        let executable = profile.executable()?;

        if !self.exe_checker.is_available(executable) {
            log::debug!("profile {name:?}: {executable:?} not found in PATH");
            return None;
        }

        log::debug!("using profile {name:?}");
        let command = profile.build_command(role).into_iter().map(String::from).collect();
        let env = profile.env.clone();
        let delimiter = self.resolve_delimiter(profile.delimiter);

        Some(SelectedPager::Pager {
            command,
            env,
            delimiter,
        })
    }

    /// Tries to use a direct command, returning `Some(SelectedPager)` if successful.
    fn try_command(&self, command: Vec<String>, source: &str) -> Option<SelectedPager> {
        let executable = command.first()?;

        if !self.exe_checker.is_available(executable) {
            log::debug!("{source}: {executable:?} not found in PATH");
            return None;
        }

        log::debug!("{source}: using as command");

        // Apply special handling for `less`
        let (command, env) = apply_less_defaults(command);
        let delimiter = self.resolve_delimiter(None);

        Some(SelectedPager::Pager {
            command,
            env,
            delimiter,
        })
    }

    /// Resolves the pager delimiter from environment variable or profile config.
    ///
    /// Precedence:
    /// 1. `HL_PAGER_DELIMITER` env var
    /// 2. Profile's `delimiter` field
    fn resolve_delimiter(&self, profile_delimiter: Option<OutputDelimiter>) -> Option<OutputDelimiter> {
        if let Some(value) = self.env_provider.get(HL_PAGER_DELIMITER) {
            match value.to_lowercase().as_str() {
                "nul" => return Some(OutputDelimiter::Nul),
                "newline" | "" => return Some(OutputDelimiter::Newline),
                other => log::warn!("{HL_PAGER_DELIMITER}: unknown value {other:?}, ignoring"),
            }
        }
        profile_delimiter
    }
}

// ---

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
            if !command.iter().any(|arg| arg == "-R" || arg.starts_with("-R")) {
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
