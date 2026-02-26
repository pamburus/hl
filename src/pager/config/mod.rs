//! Pager configuration types.
//!
//! This module defines the configuration structures for pager profiles,
//! including support for priority-based profile selection and role-specific
//! arguments (view vs follow mode).

use std::collections::HashMap;

use serde::Deserialize;

use crate::condition::{ConditionContext, Mode};
use crate::output::OutputDelimiter;

// ---

pub use crate::condition::Condition;

// ---

/// Represents a candidate in the `pager.candidates` array.
///
/// Each candidate is either an environment variable reference or a profile reference.
/// The optional `if` field imposes a condition that must be satisfied for this
/// candidate to be considered at all.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PagerCandidate {
    /// The kind of candidate (env or profile).
    #[serde(flatten)]
    pub kind: PagerCandidateKind,

    /// Optional condition; if set the candidate is only considered when it matches.
    #[serde(default)]
    pub r#if: Option<Condition>,

    /// Whether `@profile` references are supported in environment variable values.
    /// When `true`, values starting with `@` are treated as profile references.
    /// When `false` (the default), `@` is treated as a literal character in the command name.
    /// Only meaningful for env candidates; ignored for profile candidates.
    #[serde(default)]
    pub profiles: bool,
}

// ---

/// Discriminates between the two kinds of pager candidates.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PagerCandidateKind {
    /// Simple environment variable reference: `{ env = "HL_PAGER" }`
    /// or structured reference: `{ env = { pager = "...", follow = "...", delimiter = "..." } }`
    Env(EnvReference),
    /// Reference to a profile: `{ profile = "fzf" }`
    Profile(String),
}

// ---

/// Represents an environment variable reference, either simple or structured.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum EnvReference {
    /// Simple form: just a variable name
    Simple(String),
    /// Structured form with role-specific variables
    Structured(StructuredEnvReference),
}

/// Structured environment variable reference with role-specific variables.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StructuredEnvReference {
    /// Environment variable for view mode (or both modes if follow not specified).
    #[serde(default)]
    pub pager: Option<String>,

    /// Environment variable for follow mode.
    #[serde(default)]
    pub follow: Option<String>,

    /// Environment variable for delimiter override.
    #[serde(default)]
    pub delimiter: Option<String>,
}

// ---

/// Represents the top-level `pager` configuration section.
///
/// Contains a list of candidates to try in order and named pager profiles.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct PagerConfig {
    /// List of pager candidates to try in order.
    #[serde(default)]
    pub candidates: Vec<PagerCandidate>,

    /// Named pager profiles.
    #[serde(default)]
    pub profiles: Vec<PagerProfile>,
}

impl PagerConfig {
    /// Returns candidates in priority order.
    pub fn candidates(&self) -> &[PagerCandidate] {
        &self.candidates
    }

    /// Gets a profile by name.
    pub fn profile(&self, name: &str) -> Option<&PagerProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }
}

// ---

/// Represents a named pager profile.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PagerProfile {
    /// Profile name.
    pub name: String,

    /// Base command (executable): `command = "fzf"`
    #[serde(default)]
    pub command: String,

    /// Base arguments: `args = ["--ansi", "--exact"]`
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables to set: `env = { LESSCHARSET = "UTF-8" }`
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Output entry delimiter when this pager is used.
    #[serde(default)]
    pub delimiter: Option<OutputDelimiter>,

    /// Mode-specific configuration.
    #[serde(default)]
    pub modes: PagerModes,

    /// Conditional arguments based on platform and mode.
    #[serde(default)]
    pub conditions: Vec<ConditionalArgs>,
}

// ---

/// Mode-specific configuration wrapper.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct PagerModes {
    /// View mode configuration.
    #[serde(default)]
    pub view: PagerRoleConfig,

    /// Follow mode configuration.
    #[serde(default)]
    pub follow: PagerRoleConfig,
}

// ---

/// Represents conditional arguments that apply when a condition is met.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ConditionalArgs {
    /// Condition that must be met for these args to apply.
    pub r#if: Condition,

    /// Arguments to append when condition is met.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables to set when condition is met.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl PagerProfile {
    /// Returns the executable name.
    pub fn executable(&self) -> Option<&str> {
        if self.command.is_empty() {
            None
        } else {
            Some(self.command.as_str())
        }
    }

    /// Builds the full command for a given role.
    pub fn build_command(&self, role: PagerRole) -> Vec<&str> {
        let ctx = ConditionContext::from(role);
        let mut cmd = vec![self.command.as_str()];
        cmd.extend(self.args.iter().map(|s| s.as_str()));

        // Add conditional args that match current platform and mode
        for conditional in &self.conditions {
            if conditional.r#if.matches(&ctx) {
                cmd.extend(conditional.args.iter().map(|s| s.as_str()));
            }
        }

        let role_args = match role {
            PagerRole::View => &self.modes.view.args,
            PagerRole::Follow => &self.modes.follow.args,
        };
        cmd.extend(role_args.iter().map(|s| s.as_str()));
        cmd
    }

    /// Builds the environment variables for a given role.
    pub fn build_env(&self, role: PagerRole) -> HashMap<String, String> {
        let ctx = ConditionContext::from(role);
        let mut env = self.env.clone();

        // Add conditional env vars that match current platform and mode
        for conditional in &self.conditions {
            if conditional.r#if.matches(&ctx) {
                env.extend(conditional.env.clone());
            }
        }

        env
    }
}

// ---

/// Represents role-specific configuration (`view` or `follow`).
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct PagerRoleConfig {
    /// Whether pager is enabled for this role (only meaningful for follow).
    #[serde(default)]
    pub enabled: Option<bool>,

    /// Additional arguments for this role.
    #[serde(default)]
    pub args: Vec<String>,
}

impl PagerRoleConfig {
    /// Returns `true` if pager is enabled for the given role.
    ///
    /// For view: always returns `true` (implicit).
    /// For follow: only returns `true` if explicitly enabled.
    pub fn is_enabled(&self, role: PagerRole) -> bool {
        match role {
            PagerRole::View => true,
            PagerRole::Follow => self.enabled.unwrap_or(false),
        }
    }
}

// ---

/// Enum representing the context in which a pager is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagerRole {
    /// Standard log viewing (non-follow).
    View,
    /// Live log streaming (`--follow` mode).
    Follow,
}

impl From<PagerRole> for ConditionContext {
    fn from(role: PagerRole) -> Self {
        ConditionContext::with_mode(match role {
            PagerRole::View => Mode::View,
            PagerRole::Follow => Mode::Follow,
        })
    }
}
