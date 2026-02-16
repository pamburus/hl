//! Pager selection logic.
//!
//! This module provides the `PagerSelector` which handles pager selection
//! based on environment variables, configuration, and executable availability.

use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::path::Path;

use pager::{Pager, StartedPager};
use thiserror::Error;

use super::config::{EnvReference, PagerCandidate, PagerConfig, PagerRole, StructuredEnvReference};
use crate::{
    output::OutputDelimiter,
    xerr::{Highlight, HighlightQuoted},
};

// ---

/// Error that can occur during pager selection or execution.
#[derive(Error, Debug)]
pub enum Error {
    #[error("{}: profile {} does not exist in configuration", .var.hl(), .profile.hlq())]
    ProfileNotFound { var: String, profile: String },

    #[error("{}: profile {} has no command configured", .var.hl(), .profile.hlq())]
    ProfileMisconfigured { var: String, profile: String },

    #[error("{}: profile {} executable {} not found in PATH", .var.hl(), .profile.hlq(), .executable.hlq())]
    ExecutableNotFound {
        var: String,
        profile: String,
        executable: String,
    },

    #[error("{var}: empty command", var=.var.hl())]
    EmptyCommand { var: String },

    #[error("{}: command {} not found in PATH", .var.hl(), .command.hlq())]
    CommandNotFound { var: String, command: String },

    #[error("failed to start pager {}: {source}", quote_command(.command).hlq())]
    StartFailed {
        command: Vec<String>,
        #[source]
        source: std::io::Error,
    },

    #[error("pager {} exited with code {exit_code}", quote_command(.command).hlq())]
    PagerFailed { command: Vec<String>, exit_code: i32 },

    #[error("failed to wait for pager process: {source}")]
    WaitFailed {
        #[source]
        source: std::io::Error,
    },
}

fn quote_command(command: &[String]) -> String {
    shellwords::join(&command.iter().map(String::as_str).collect::<Vec<_>>())
}

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

impl SelectedPager {
    /// Starts the selected pager process.
    ///
    /// Returns `Ok(Some((started_pager, delimiter)))` if a pager was selected and started,
    /// or `Ok(None)` if no pager was selected.
    pub fn start(self) -> Result<Option<(StartedPager, Option<OutputDelimiter>)>, Error> {
        match self {
            SelectedPager::Pager {
                command,
                env,
                delimiter,
            } => {
                let started = Pager::custom(&command)
                    .with_env(env)
                    .start()
                    .map(|r| r.map_err(|source| Error::StartFailed { command, source }))
                    .transpose()?;
                Ok(started.map(|pager| (pager, delimiter)))
            }
            SelectedPager::None => Ok(None),
        }
    }
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
    /// The pager configuration.
    config: &'a PagerConfig,
    /// Environment provider.
    env_provider: E,
    /// Executable checker.
    exe_checker: C,
}

impl<'a> PagerSelector<'a, SystemEnv, SystemExeChecker> {
    /// Creates a new pager selector with the given configuration.
    pub fn new(config: &'a PagerConfig) -> Self {
        Self {
            config,
            env_provider: SystemEnv,
            exe_checker: SystemExeChecker,
        }
    }
}

impl<'a, E: EnvProvider, C: ExeChecker> PagerSelector<'a, E, C> {
    /// Creates a new pager selector with custom environment and executable checker.
    pub fn with_providers(config: &'a PagerConfig, env_provider: E, exe_checker: C) -> Self {
        Self {
            config,
            env_provider,
            exe_checker,
        }
    }

    /// Selects a pager for the given role.
    pub fn select(&self, role: PagerRole) -> Result<SelectedPager, Error> {
        log::debug!("selecting pager for {role:?} mode");
        let result = match role {
            PagerRole::View => self.select_for_view(),
            PagerRole::Follow => self.select_for_follow(),
        };
        if let Ok(selected) = &result {
            match selected {
                SelectedPager::Pager { command, .. } => {
                    log::debug!("selected pager: {command:?}");
                }
                SelectedPager::None => {
                    log::debug!("no pager selected, using stdout");
                }
            }
        }
        result
    }

    /// Selects a pager for view mode.
    ///
    /// Iterates through pager.candidates in order until a usable pager is found.
    fn select_for_view(&self) -> Result<SelectedPager, Error> {
        for candidate in self.config.candidates() {
            match candidate {
                PagerCandidate::Profile(name) => {
                    if let Some(selected) = self.try_profile(name, PagerRole::View) {
                        return Ok(selected);
                    }
                }
                PagerCandidate::Env(env_ref) => {
                    if let Some(result) = self.try_env_candidate(env_ref, PagerRole::View)? {
                        return Ok(result);
                    }
                }
            }
        }

        log::debug!("no pager configured, using stdout");
        Ok(SelectedPager::None)
    }

    /// Selects a pager for follow mode.
    ///
    /// Iterates through pager.candidates in order until a usable pager is found.
    /// Only uses profiles that have follow.enabled = true.
    fn select_for_follow(&self) -> Result<SelectedPager, Error> {
        for candidate in self.config.candidates() {
            match candidate {
                PagerCandidate::Profile(name) => {
                    // Only use profiles with follow.enabled = true
                    if let Some(profile) = self.config.profile(name) {
                        if profile.follow.is_enabled(PagerRole::Follow) {
                            if let Some(selected) = self.try_profile(name, PagerRole::Follow) {
                                return Ok(selected);
                            }
                        }
                    }
                }
                PagerCandidate::Env(env_ref) => {
                    if let Some(result) = self.try_env_candidate(env_ref, PagerRole::Follow)? {
                        return Ok(result);
                    }
                }
            }
        }

        log::debug!("no pager configured for follow mode, using stdout");
        Ok(SelectedPager::None)
    }

    /// Tries to resolve an environment variable candidate.
    ///
    /// Returns:
    /// - `Ok(Some(SelectedPager))` if a pager was successfully selected
    /// - `Ok(None)` if the candidate should be skipped (env var not set, command not available for profile refs)
    /// - `Err(Error)` if there's a fatal error (command not available for direct commands or @profile refs)
    fn try_env_candidate(&self, env_ref: &EnvReference, role: PagerRole) -> Result<Option<SelectedPager>, Error> {
        match env_ref {
            EnvReference::Simple(var_name) => self.try_simple_env_candidate(var_name, role),
            EnvReference::Structured(structured) => self.try_structured_env_candidate(structured, role),
        }
    }

    /// Tries to resolve a simple environment variable candidate.
    fn try_simple_env_candidate(&self, var_name: &str, role: PagerRole) -> Result<Option<SelectedPager>, Error> {
        let value = match self.env_provider.get(var_name) {
            Some(v) if !v.is_empty() => v,
            _ => {
                log::debug!("env var {var_name} not set or empty, skipping candidate");
                return Ok(None);
            }
        };

        // Parse the value
        let parts = self.parse_command(&value, var_name)?;

        // Check if it's a profile reference
        if let Some(profile_name) = parts.first().and_then(|s| s.strip_prefix('@')) {
            log::debug!("env var {var_name} references profile: {profile_name}");
            return self.resolve_profile_reference(profile_name, role, var_name);
        }

        // It's a direct command - resolve it
        self.resolve_direct_command(&parts, role, var_name, None)
    }

    /// Tries to resolve a structured environment variable candidate.
    fn try_structured_env_candidate(
        &self,
        structured: &StructuredEnvReference,
        role: PagerRole,
    ) -> Result<Option<SelectedPager>, Error> {
        // Determine which env var to use based on role
        let (var_name, is_follow_explicit) = match role {
            PagerRole::View => (structured.pager.as_deref(), false),
            PagerRole::Follow => {
                // For follow mode, check follow field first
                if let Some(follow_var) = structured.follow.as_deref() {
                    (Some(follow_var), true)
                } else {
                    // Fall back to pager field, but it's not explicit for follow
                    (structured.pager.as_deref(), false)
                }
            }
        };

        let var_name = match var_name {
            Some(name) => name,
            None => {
                log::debug!("structured env candidate has no var for {role:?} mode, skipping");
                return Ok(None);
            }
        };

        let value = match self.env_provider.get(var_name) {
            Some(v) if !v.is_empty() => v,
            _ => {
                log::debug!("env var {var_name} not set or empty, skipping candidate");
                // If follow field was specified but not set, and we're in follow mode, disable paging
                if role == PagerRole::Follow && is_follow_explicit {
                    log::debug!("follow field specified but not set, disabling paging for follow mode");
                    return Ok(Some(SelectedPager::None));
                }
                return Ok(None);
            }
        };

        // Parse the value
        let parts = self.parse_command(&value, var_name)?;

        // Check if it's a profile reference
        if let Some(profile_name) = parts.first().and_then(|s| s.strip_prefix('@')) {
            log::debug!("env var {var_name} references profile: {profile_name}");
            // Profile reference - ignore delimiter field
            return self.resolve_profile_reference(profile_name, role, var_name);
        }

        // It's a direct command
        // For follow mode: only use if explicitly set via follow field
        if role == PagerRole::Follow && !is_follow_explicit {
            log::debug!("direct command in follow mode without explicit follow field, disabling paging");
            return Ok(Some(SelectedPager::None));
        }

        // Check delimiter field
        let delimiter = structured
            .delimiter
            .as_deref()
            .and_then(|var| self.env_provider.get(var))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .and_then(|v| match v.to_lowercase().as_str() {
                "nul" => Some(OutputDelimiter::Nul),
                "newline" => Some(OutputDelimiter::Newline),
                _ => {
                    log::warn!("delimiter: unknown value {v:?}, ignoring");
                    None
                }
            });

        self.resolve_direct_command(&parts, role, var_name, delimiter)
    }

    /// Parses a command string using shellwords.
    fn parse_command(&self, value: &str, var_name: &str) -> Result<Vec<String>, Error> {
        match shellwords::split(value) {
            Ok(parts) if !parts.is_empty() => Ok(parts),
            Ok(_) => Err(Error::EmptyCommand {
                var: var_name.to_owned(),
            }),
            Err(e) => {
                log::warn!("{var_name}: failed to parse with shellwords: {e}, using raw value");
                Ok(vec![value.to_owned()])
            }
        }
    }

    /// Resolves a profile reference (from @profile syntax).
    ///
    /// Returns:
    /// - `Ok(Some(SelectedPager))` if profile exists and command is available
    /// - `Ok(None)` if profile doesn't exist or command not available (for profile candidates)
    /// - `Err(Error)` if profile doesn't exist or command not available (for env var references)
    fn resolve_profile_reference(
        &self,
        profile_name: &str,
        role: PagerRole,
        var_name: &str,
    ) -> Result<Option<SelectedPager>, Error> {
        let profile = self
            .config
            .profile(profile_name)
            .ok_or_else(|| Error::ProfileNotFound {
                var: var_name.to_owned(),
                profile: profile_name.to_owned(),
            })?;

        let executable = profile.executable().ok_or_else(|| Error::ProfileMisconfigured {
            var: var_name.to_owned(),
            profile: profile_name.to_owned(),
        })?;

        if !self.exe_checker.is_available(executable) {
            return Err(Error::ExecutableNotFound {
                var: var_name.to_owned(),
                profile: profile_name.to_owned(),
                executable: executable.to_owned(),
            });
        }

        // Check if profile supports this role (for follow mode)
        if role == PagerRole::Follow && !profile.follow.is_enabled(role) {
            log::debug!("profile {profile_name:?} does not support follow mode");
            return Ok(Some(SelectedPager::None));
        }

        log::debug!("using profile {profile_name:?}");
        let command = profile.build_command(role).into_iter().map(String::from).collect();
        let env = profile.env.clone();
        // Profile delimiter takes precedence - env.delimiter is ignored for profile references
        let delimiter = profile.delimiter;

        Ok(Some(SelectedPager::Pager {
            command,
            env,
            delimiter,
        }))
    }

    /// Resolves a direct command (not a profile reference).
    ///
    /// Returns:
    /// - `Ok(Some(SelectedPager))` if command is available
    /// - `Err(Error)` if command is not available
    fn resolve_direct_command(
        &self,
        parts: &[String],
        _role: PagerRole,
        var_name: &str,
        delimiter: Option<OutputDelimiter>,
    ) -> Result<Option<SelectedPager>, Error> {
        let executable = parts.first().ok_or_else(|| Error::EmptyCommand {
            var: var_name.to_owned(),
        })?;

        if !self.exe_checker.is_available(executable) {
            return Err(Error::CommandNotFound {
                var: var_name.to_owned(),
                command: executable.to_owned(),
            });
        }

        log::debug!("{var_name}: using as direct command");

        // Apply special handling for `less`
        let (command, env) = apply_less_defaults(parts.to_vec());

        Ok(Some(SelectedPager::Pager {
            command,
            env,
            delimiter,
        }))
    }

    /// Tries to use a named profile, returning `Some(SelectedPager)` if successful.
    fn try_profile(&self, name: &str, role: PagerRole) -> Option<SelectedPager> {
        let profile = self.config.profile(name)?;

        let executable = profile.executable()?;

        if !self.exe_checker.is_available(executable) {
            log::debug!("profile {name:?}: {executable:?} not found in PATH");
            return None;
        }

        // Check if profile supports this role (for follow mode)
        if role == PagerRole::Follow && !profile.follow.is_enabled(role) {
            log::debug!("profile {name:?} does not support follow mode");
            return None;
        }

        log::debug!("using profile {name:?}");
        let command = profile.build_command(role).into_iter().map(String::from).collect();
        let env = profile.env.clone();
        let delimiter = profile.delimiter;

        Some(SelectedPager::Pager {
            command,
            env,
            delimiter,
        })
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
