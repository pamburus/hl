//! Pager configuration types.
//!
//! This module defines the configuration structures for pager profiles,
//! including support for priority-based profile selection and role-specific
//! arguments (view vs follow mode).

use std::collections::HashMap;

use serde::Deserialize;

// ---

/// Represents the top-level `pager` configuration option.
///
/// Can be either a single profile name or a priority-ordered list of profiles.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum PagerConfig {
    /// Single profile name: `pager = "fzf"`
    Single(String),
    /// Priority list: `pager = ["fzf", "less"]`
    Priority(Vec<String>),
}

impl PagerConfig {
    /// Returns profile names in priority order.
    pub fn profiles(&self) -> impl Iterator<Item = &str> {
        let iter: Box<dyn Iterator<Item = &str>> = match self {
            PagerConfig::Single(name) => Box::new(std::iter::once(name.as_str())),
            PagerConfig::Priority(names) => Box::new(names.iter().map(|s| s.as_str())),
        };
        iter
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
