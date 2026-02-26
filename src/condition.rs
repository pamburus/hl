//! Condition parsing for platform and mode-based conditional configuration.
//!
//! Supports conditions in the format:
//! - `os:macos`, `os:linux`, `os:windows`, `os:unix`
//! - `mode:view`, `mode:follow`
//! - `!<condition>` for negation
//!
//! Examples:
//! - `os:macos` - matches on macOS
//! - `!mode:follow` - matches when NOT in follow mode
//! - `os:unix` - matches on Unix-like systems (macOS or Linux)

use std::fmt;
use std::str::FromStr;

use itertools::Itertools;
use serde::{Deserialize, Deserializer};
use strum::{AsRefStr, EnumIter, IntoEnumIterator};
use thiserror::Error;

use crate::xerr::{Highlight, HighlightQuoted};

// ---

/// Context for evaluating conditions.
///
/// Callers populate this from their domain-specific state before calling
/// [`Condition::matches`].  Fields that are not relevant to a particular
/// context are left as `None`, which causes the corresponding condition
/// variants to evaluate to `false`.
#[derive(Debug, Clone, Default)]
pub struct ConditionContext {
    /// The current mode, if any.
    pub mode: Option<Mode>,
}

impl ConditionContext {
    /// Creates a context with the given mode.
    pub fn with_mode(mode: Mode) -> Self {
        Self { mode: Some(mode) }
    }
}

// ---

/// Represents a mode for condition evaluation.
///
/// Callers convert their own mode representation to this enum before building
/// a [`ConditionContext`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// View mode (non-follow).
    View,
    /// Follow mode.
    Follow,
}

// ---

/// Represents a condition that can be evaluated against a [`ConditionContext`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// OS-based condition.
    Os(OsCondition),
    /// Mode-based condition.
    Mode(ModeCondition),
    /// Negation of a condition.
    Not(Box<Condition>),
}

impl Condition {
    pub const PREFIXES: [&'static str; 2] = ["os", "mode"];

    /// Evaluates whether this condition matches the given context.
    pub fn matches(&self, ctx: &ConditionContext) -> bool {
        match self {
            Condition::Os(os) => os.matches(),
            Condition::Mode(mode) => mode.matches(ctx),
            Condition::Not(cond) => !cond.matches(ctx),
        }
    }
}

impl FromStr for Condition {
    type Err = ConditionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Check for negation prefix
        if let Some(inner) = s.strip_prefix('!') {
            let inner_cond = inner.trim().parse()?;
            return Ok(Condition::Not(Box::new(inner_cond)));
        }

        // Parse os: or mode: prefix
        if let Some((prefix, value)) = s.split_once(':') {
            match prefix {
                "os" => {
                    let os_cond = value.parse()?;
                    Ok(Condition::Os(os_cond))
                }
                "mode" => {
                    let mode_cond = value.parse()?;
                    Ok(Condition::Mode(mode_cond))
                }
                _ => Err(ConditionError::UnknownPrefix(prefix.to_string())),
            }
        } else {
            Err(ConditionError::MissingPrefix(s.to_string()))
        }
    }
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Condition::Os(os) => write!(f, "os:{}", os),
            Condition::Mode(mode) => write!(f, "mode:{}", mode),
            Condition::Not(cond) => write!(f, "!{}", cond),
        }
    }
}

// ---

/// OS-based condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, AsRefStr)]
pub enum OsCondition {
    /// macOS
    MacOS,
    /// Linux
    Linux,
    /// Windows
    Windows,
    /// Unix-like (macOS or Linux)
    Unix,
}

impl OsCondition {
    /// Evaluates whether this OS condition matches the current platform.
    pub fn matches(&self) -> bool {
        match self {
            OsCondition::MacOS => cfg!(target_os = "macos"),
            OsCondition::Linux => cfg!(target_os = "linux"),
            OsCondition::Windows => cfg!(target_os = "windows"),
            OsCondition::Unix => cfg!(unix),
        }
    }
}

impl FromStr for OsCondition {
    type Err = ConditionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "macos" => Ok(OsCondition::MacOS),
            "linux" => Ok(OsCondition::Linux),
            "windows" => Ok(OsCondition::Windows),
            "unix" => Ok(OsCondition::Unix),
            _ => Err(ConditionError::UnknownOs(s.to_string())),
        }
    }
}

impl fmt::Display for OsCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsCondition::MacOS => write!(f, "macos"),
            OsCondition::Linux => write!(f, "linux"),
            OsCondition::Windows => write!(f, "windows"),
            OsCondition::Unix => write!(f, "unix"),
        }
    }
}

// ---

/// Mode-based condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, AsRefStr)]
pub enum ModeCondition {
    /// View mode (non-follow)
    View,
    /// Follow mode
    Follow,
}

impl ModeCondition {
    /// Evaluates whether this mode condition matches the given context.
    ///
    /// Returns `false` if the context carries no mode information.
    pub fn matches(&self, ctx: &ConditionContext) -> bool {
        match ctx.mode {
            Some(Mode::View) => *self == ModeCondition::View,
            Some(Mode::Follow) => *self == ModeCondition::Follow,
            None => false,
        }
    }
}

impl FromStr for ModeCondition {
    type Err = ConditionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "view" => Ok(ModeCondition::View),
            "follow" => Ok(ModeCondition::Follow),
            _ => Err(ConditionError::UnknownMode(s.to_string())),
        }
    }
}

impl fmt::Display for ModeCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModeCondition::View => write!(f, "view"),
            ModeCondition::Follow => write!(f, "follow"),
        }
    }
}

// ---

/// Error type for condition parsing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConditionError {
    /// Unknown prefix (expected "os" or "mode")
    #[error("unknown condition prefix {} (valid: {})", .0.hlq(), Condition::PREFIXES.hl())]
    UnknownPrefix(String),

    /// Missing prefix separator ':'
    #[error("condition {} must have one of {} prefixes", .0.hlq(), Condition::PREFIXES.hl())]
    MissingPrefix(String),

    /// Unknown OS value
    #[error("unknown os {} (valid: {})", .0.hlq(), OsCondition::iter().collect_vec().hl())]
    UnknownOs(String),

    /// Unknown mode value
    #[error("unknown mode {} (valid: {})", .0.hlq(), ModeCondition::iter().collect_vec().hl())]
    UnknownMode(String),
}

// ---

#[cfg(test)]
mod tests;
