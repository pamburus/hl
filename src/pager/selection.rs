//! Pager selection logic.
//!
//! This module provides the `PagerSelector` which handles pager selection
//! based on environment variables, configuration, and executable availability.

use std::collections::HashMap;
use std::env;

use pager::{Pager, StartedPager};
use thiserror::Error;

use crate::condition::ConditionContext;

use super::config::{
    EnvReference, PagerCandidate, PagerCandidateKind, PagerConfig, PagerProfile, PagerRole, StructuredEnvReference,
};
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
        let result = self.select_candidates(role);
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

    /// Iterates through pager candidates in order until a usable pager is found.
    fn select_candidates(&self, role: PagerRole) -> Result<SelectedPager, Error> {
        let ctx = ConditionContext::from(role);
        for candidate in self.config.candidates() {
            if !self.candidate_matches_condition(candidate, &ctx) {
                continue;
            }
            match &candidate.kind {
                PagerCandidateKind::Profile(name) => {
                    if let Some(selected) = self.try_profile(name, role) {
                        return Ok(selected);
                    }
                }
                PagerCandidateKind::Env(env_ref) => {
                    if let Some(result) = self.try_env_candidate(env_ref, role, candidate.profiles)? {
                        return Ok(result);
                    }
                }
            }
        }

        log::debug!("no pager configured, using stdout");
        Ok(SelectedPager::None)
    }

    /// Returns `true` if the candidate's `if` condition is satisfied (or absent).
    fn candidate_matches_condition(&self, candidate: &PagerCandidate, ctx: &ConditionContext) -> bool {
        match &candidate.r#if {
            Some(cond) => {
                let result = cond.matches(ctx);
                if !result {
                    log::debug!("candidate {:?} skipped: condition {cond} not met", candidate.kind);
                }
                result
            }
            None => true,
        }
    }

    /// Tries to resolve an environment variable candidate.
    ///
    /// Returns:
    /// - `Ok(Some(SelectedPager))` if a pager was successfully selected
    /// - `Ok(None)` if the candidate should be skipped (env var not set, command not available for profile refs)
    /// - `Err(Error)` if there's a fatal error (command not available for direct commands or @profile refs)
    fn try_env_candidate(
        &self,
        env_ref: &EnvReference,
        role: PagerRole,
        profiles: bool,
    ) -> Result<Option<SelectedPager>, Error> {
        match env_ref {
            EnvReference::Simple(var_name) => self.try_simple_env_candidate(var_name, role, profiles),
            EnvReference::Structured(structured) => self.try_structured_env_candidate(structured, role, profiles),
        }
    }

    /// If `profiles` is enabled and `parts[0]` starts with `@`, resolves it as a profile
    /// reference and returns `Some(result)`. Returns `None` if it is not a profile reference.
    fn try_resolve_profile_ref(
        &self,
        executable: &str,
        role: PagerRole,
        var_name: &str,
        profiles: bool,
    ) -> Option<Result<Option<SelectedPager>, Error>> {
        if !profiles {
            return None;
        }
        let profile_name = executable.strip_prefix('@')?;
        log::debug!("env var {var_name} references profile: {profile_name}");
        Some(self.resolve_profile_reference(profile_name, role, var_name))
    }

    /// Tries to resolve a simple environment variable candidate.
    fn try_simple_env_candidate(
        &self,
        var_name: &str,
        role: PagerRole,
        profiles: bool,
    ) -> Result<Option<SelectedPager>, Error> {
        // Simple env candidates are view-only.
        if role == PagerRole::Follow {
            log::debug!("simple env candidate {var_name} skipped: not supported in follow mode");
            return Ok(None);
        }

        let value = match self.env_provider.get(var_name) {
            Some(v) if !v.is_empty() => v,
            _ => {
                log::debug!("env var {var_name} not set or empty, skipping candidate");
                return Ok(None);
            }
        };

        self.resolve_command_value(&value, var_name, role, profiles, None)
    }

    /// Tries to resolve a structured environment variable candidate.
    ///
    /// `HL_PAGER` (the `pager` field) is the sole candidate-selection gate:
    /// - If it is not set or empty the whole candidate is skipped.
    /// - If it holds a profile reference (`@name`) the profile is resolved and all
    ///   other fields (`follow`, `delimiter`) are ignored entirely.
    /// - If it holds a direct command, mode-specific handling applies:
    ///   - View: uses the command with an optional delimiter from `HL_PAGER_DELIMITER`.
    ///   - Follow: reads `HL_FOLLOW_PAGER`; if set uses it as the command; if not set
    ///     paging is disabled for this invocation.
    fn try_structured_env_candidate(
        &self,
        structured: &StructuredEnvReference,
        role: PagerRole,
        profiles: bool,
    ) -> Result<Option<SelectedPager>, Error> {
        // HL_PAGER is the sole candidate-selection gate.
        let pager_var = match structured.pager.as_deref() {
            Some(v) => v,
            None => {
                log::debug!("structured env candidate has no pager var, skipping");
                return Ok(None);
            }
        };

        let pager_value = match self.env_provider.get(pager_var) {
            Some(v) if !v.is_empty() => v,
            _ => {
                log::debug!("env var {pager_var} not set or empty, skipping candidate");
                return Ok(None);
            }
        };

        let (executable, args) = self.parse_command(&pager_value, pager_var)?;

        // Profile references: all other fields (follow var, delimiter var) are ignored.
        if let Some(result) = self.try_resolve_profile_ref(&executable, role, pager_var, profiles) {
            return result;
        }

        // Delimiter applies to both modes (controls the output format fed to the pager).
        let delimiter = self.resolve_env_delimiter(structured);

        // Direct command — apply mode-specific handling.
        match role {
            PagerRole::View => self.resolve_direct_command(&executable, &args, pager_var, delimiter),
            PagerRole::Follow => {
                // HL_FOLLOW_PAGER determines the follow-mode command.
                // If it is not configured or not set, paging is disabled.
                match structured.follow.as_deref() {
                    None => {
                        log::debug!("{pager_var}: direct command has no follow field, disabling paging");
                        Ok(Some(SelectedPager::None))
                    }
                    Some(follow_var) => match self.env_provider.get(follow_var).filter(|v| !v.is_empty()) {
                        None => {
                            log::debug!("{follow_var}: not set, disabling paging in follow mode");
                            Ok(Some(SelectedPager::None))
                        }
                        Some(follow_value) => {
                            self.resolve_command_value(&follow_value, follow_var, role, profiles, delimiter)
                        }
                    },
                }
            }
        }
    }

    /// Parses a raw command value, checks for a profile reference, and resolves the result.
    ///
    /// Shared by simple env candidates and the follow arm of structured env candidates.
    fn resolve_command_value(
        &self,
        value: &str,
        var_name: &str,
        role: PagerRole,
        profiles: bool,
        delimiter: Option<OutputDelimiter>,
    ) -> Result<Option<SelectedPager>, Error> {
        let (executable, args) = self.parse_command(value, var_name)?;
        if let Some(result) = self.try_resolve_profile_ref(&executable, role, var_name, profiles) {
            return result;
        }
        self.resolve_direct_command(&executable, &args, var_name, delimiter)
    }

    /// Resolves the delimiter from the structured env candidate's delimiter field.
    fn resolve_env_delimiter(&self, structured: &StructuredEnvReference) -> Option<OutputDelimiter> {
        structured
            .delimiter
            .as_deref()
            .and_then(|var| self.env_provider.get(var))
            .filter(|v| !v.is_empty())
            .and_then(|v| match v.to_lowercase().as_str() {
                "nul" => Some(OutputDelimiter::Nul),
                "newline" => Some(OutputDelimiter::Newline),
                _ => {
                    log::warn!("delimiter: unknown value {v:?}, ignoring");
                    None
                }
            })
    }

    /// Parses a command string using shellwords into executable and arguments.
    fn parse_command(&self, value: &str, var_name: &str) -> Result<(String, Vec<String>), Error> {
        match shellwords::split(value) {
            Ok(parts) => {
                let mut iter = parts.into_iter();
                let executable = iter.next().ok_or_else(|| Error::EmptyCommand {
                    var: var_name.to_owned(),
                })?;
                Ok((executable, iter.collect()))
            }
            Err(e) => {
                log::warn!("{var_name}: failed to parse with shellwords: {e}, using raw value");
                Ok((value.to_owned(), vec![]))
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
        if role == PagerRole::Follow && !profile.modes.follow.is_enabled(role) {
            log::debug!("profile {profile_name:?} does not support follow mode");
            return Ok(Some(SelectedPager::None));
        }

        Ok(Some(selected_pager_from_profile(profile, profile_name, role)))
    }

    /// Resolves a direct command (not a profile reference).
    ///
    /// Returns:
    /// - `Ok(Some(SelectedPager))` if command is available
    /// - `Err(Error)` if command is not available
    fn resolve_direct_command(
        &self,
        executable: &str,
        args: &[String],
        var_name: &str,
        delimiter: Option<OutputDelimiter>,
    ) -> Result<Option<SelectedPager>, Error> {
        if !self.exe_checker.is_available(executable) {
            return Err(Error::CommandNotFound {
                var: var_name.to_owned(),
                command: executable.to_owned(),
            });
        }

        log::debug!("{var_name}: using as direct command");

        let command = std::iter::once(executable.to_owned())
            .chain(args.iter().cloned())
            .collect();

        Ok(Some(SelectedPager::Pager {
            command,
            env: HashMap::new(),
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

        // Profile is selected. If it does not support follow mode, disable paging rather
        // than falling through to a lower-priority candidate.
        if role == PagerRole::Follow && !profile.modes.follow.is_enabled(role) {
            log::debug!("profile {name:?} does not support follow mode, disabling pager");
            return Some(SelectedPager::None);
        }

        Some(selected_pager_from_profile(profile, name, role))
    }
}

fn selected_pager_from_profile(profile: &PagerProfile, name: &str, role: PagerRole) -> SelectedPager {
    log::debug!("using profile {name:?}");
    let command = profile.build_command(role).into_iter().map(String::from).collect();
    let env = profile.env.clone();
    let delimiter = profile.delimiter;
    SelectedPager::Pager {
        command,
        env,
        delimiter,
    }
}

// ---

/// Checks if an executable is available in PATH.
pub fn is_available(executable: &str) -> bool {
    which::which(executable).is_ok()
}

// ---
