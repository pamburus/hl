//! Pager configuration types.
//!
//! This module defines the configuration structures for pager profiles,
//! including support for priority-based profile selection and role-specific
//! arguments (view vs follow mode).

use std::collections::HashMap;

use serde::Deserialize;

use crate::output::OutputDelimiter;

// ---

/// Represents a candidate in the `pager.candidates` array.
///
/// Each candidate is either an environment variable reference or a profile reference.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PagerCandidate {
    /// Reference to an environment variable: `{ env = "HL_PAGER" }`
    Env(String),
    /// Reference to a profile: `{ profile = "fzf" }`
    Profile(String),
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
    pub profiles: HashMap<String, PagerProfile>,
}

impl PagerConfig {
    /// Returns candidates in priority order.
    pub fn candidates(&self) -> &[PagerCandidate] {
        &self.candidates
    }

    /// Gets a profile by name.
    pub fn profile(&self, name: &str) -> Option<&PagerProfile> {
        self.profiles.get(name)
    }
}

// ---

/// Represents a named pager profile in the `[pagers.<name>]` section.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PagerProfile {
    /// Base command and arguments: `command = ["fzf", "--ansi"]`
    #[serde(default)]
    pub command: Vec<String>,

    /// Environment variables to set: `env = { LESSCHARSET = "UTF-8" }`
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Output entry delimiter when this pager is used.
    #[serde(default)]
    pub delimiter: Option<OutputDelimiter>,

    /// View mode configuration.
    #[serde(default)]
    pub view: PagerRoleConfig,

    /// Follow mode configuration.
    #[serde(default)]
    pub follow: PagerRoleConfig,
}

impl PagerProfile {
    /// Returns the executable name (first element of command).
    pub fn executable(&self) -> Option<&str> {
        self.command.first().map(|s| s.as_str())
    }

    /// Builds the full command for a given role.
    pub fn build_command(&self, role: PagerRole) -> Vec<&str> {
        let mut cmd: Vec<&str> = self.command.iter().map(|s| s.as_str()).collect();
        let args = match role {
            PagerRole::View => &self.view.args,
            PagerRole::Follow => &self.follow.args,
        };
        cmd.extend(args.iter().map(|s| s.as_str()));
        cmd
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
