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

use serde::{Deserialize, Deserializer};
use thiserror::Error;

use super::PagerRole;

// ---

/// Represents a condition that can be evaluated against the current platform and mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// OS-based condition
    Os(OsCondition),
    /// Mode-based condition
    Mode(ModeCondition),
    /// Negation of a condition
    Not(Box<Condition>),
}

impl Condition {
    /// Evaluates whether this condition matches the current platform and role.
    pub fn matches(&self, role: PagerRole) -> bool {
        match self {
            Condition::Os(os) => os.matches(),
            Condition::Mode(mode) => mode.matches(role),
            Condition::Not(cond) => !cond.matches(role),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeCondition {
    /// View mode (non-follow)
    View,
    /// Follow mode
    Follow,
}

impl ModeCondition {
    /// Evaluates whether this mode condition matches the given role.
    pub fn matches(&self, role: PagerRole) -> bool {
        matches!(
            (self, role),
            (ModeCondition::View, PagerRole::View) | (ModeCondition::Follow, PagerRole::Follow)
        )
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
    #[error("unknown condition prefix '{0}' (expected 'os' or 'mode')")]
    UnknownPrefix(String),

    /// Missing prefix separator ':'
    #[error("condition '{0}' must have 'os:' or 'mode:' prefix")]
    MissingPrefix(String),

    /// Unknown OS value
    #[error("unknown OS '{0}' (expected 'macos', 'linux', 'windows', or 'unix')")]
    UnknownOs(String),

    /// Unknown mode value
    #[error("unknown mode '{0}' (expected 'view' or 'follow')")]
    UnknownMode(String),
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_parse_os_macos() {
        let cond: Condition = "os:macos".parse().expect("failed to parse");
        assert_eq!(cond, Condition::Os(OsCondition::MacOS));
        assert_eq!(cond.to_string(), "os:macos");
    }

    #[test]
    fn condition_parse_os_linux() {
        let cond: Condition = "os:linux".parse().expect("failed to parse");
        assert_eq!(cond, Condition::Os(OsCondition::Linux));
        assert_eq!(cond.to_string(), "os:linux");
    }

    #[test]
    fn condition_parse_os_windows() {
        let cond: Condition = "os:windows".parse().expect("failed to parse");
        assert_eq!(cond, Condition::Os(OsCondition::Windows));
        assert_eq!(cond.to_string(), "os:windows");
    }

    #[test]
    fn condition_parse_os_unix() {
        let cond: Condition = "os:unix".parse().expect("failed to parse");
        assert_eq!(cond, Condition::Os(OsCondition::Unix));
        assert_eq!(cond.to_string(), "os:unix");
    }

    #[test]
    fn condition_parse_mode_view() {
        let cond: Condition = "mode:view".parse().expect("failed to parse");
        assert_eq!(cond, Condition::Mode(ModeCondition::View));
        assert_eq!(cond.to_string(), "mode:view");
    }

    #[test]
    fn condition_parse_mode_follow() {
        let cond: Condition = "mode:follow".parse().expect("failed to parse");
        assert_eq!(cond, Condition::Mode(ModeCondition::Follow));
        assert_eq!(cond.to_string(), "mode:follow");
    }

    #[test]
    fn condition_parse_negation() {
        let cond: Condition = "!mode:follow".parse().expect("failed to parse");
        assert_eq!(cond, Condition::Not(Box::new(Condition::Mode(ModeCondition::Follow))));
        assert_eq!(cond.to_string(), "!mode:follow");
    }

    #[test]
    fn condition_parse_negation_with_spaces() {
        let cond: Condition = "! mode:view".parse().expect("failed to parse");
        assert_eq!(cond, Condition::Not(Box::new(Condition::Mode(ModeCondition::View))));
    }

    #[test]
    fn condition_parse_with_whitespace() {
        let cond: Condition = "  os:macos  ".parse().expect("failed to parse");
        assert_eq!(cond, Condition::Os(OsCondition::MacOS));
    }

    #[test]
    fn condition_parse_unknown_os() {
        let result: Result<Condition, _> = "os:freebsd".parse();
        assert!(matches!(result, Err(ConditionError::UnknownOs(_))));
    }

    #[test]
    fn condition_parse_unknown_mode() {
        let result: Result<Condition, _> = "mode:stream".parse();
        assert!(matches!(result, Err(ConditionError::UnknownMode(_))));
    }

    #[test]
    fn condition_parse_unknown_prefix() {
        let result: Result<Condition, _> = "arch:x86_64".parse();
        assert!(matches!(result, Err(ConditionError::UnknownPrefix(_))));
    }

    #[test]
    fn condition_parse_missing_prefix() {
        let result: Result<Condition, _> = "macos".parse();
        assert!(matches!(result, Err(ConditionError::MissingPrefix(_))));
    }

    #[test]
    fn condition_matches_os() {
        #[cfg(target_os = "macos")]
        {
            let cond = Condition::Os(OsCondition::MacOS);
            assert!(cond.matches(PagerRole::View));
            assert!(cond.matches(PagerRole::Follow));
        }

        #[cfg(target_os = "linux")]
        {
            let cond = Condition::Os(OsCondition::Linux);
            assert!(cond.matches(PagerRole::View));
            assert!(cond.matches(PagerRole::Follow));
        }
    }

    #[test]
    fn condition_matches_unix() {
        #[cfg(unix)]
        {
            let cond = Condition::Os(OsCondition::Unix);
            assert!(cond.matches(PagerRole::View));
            assert!(cond.matches(PagerRole::Follow));
        }

        #[cfg(not(unix))]
        {
            let cond = Condition::Os(OsCondition::Unix);
            assert!(!cond.matches(PagerRole::View));
            assert!(!cond.matches(PagerRole::Follow));
        }
    }

    #[test]
    fn condition_matches_mode_view() {
        let cond = Condition::Mode(ModeCondition::View);
        assert!(cond.matches(PagerRole::View));
        assert!(!cond.matches(PagerRole::Follow));
    }

    #[test]
    fn condition_matches_mode_follow() {
        let cond = Condition::Mode(ModeCondition::Follow);
        assert!(!cond.matches(PagerRole::View));
        assert!(cond.matches(PagerRole::Follow));
    }

    #[test]
    fn condition_matches_negation() {
        let cond = Condition::Not(Box::new(Condition::Mode(ModeCondition::Follow)));
        assert!(cond.matches(PagerRole::View));
        assert!(!cond.matches(PagerRole::Follow));
    }

    #[test]
    fn condition_deserialize_from_toml() {
        #[derive(Deserialize)]
        struct TestConfig {
            when: Condition,
        }

        let toml = r#"when = "os:macos""#;
        let config: TestConfig = toml::from_str(toml).expect("failed to parse");
        assert_eq!(config.when, Condition::Os(OsCondition::MacOS));

        let toml = r#"when = "!mode:follow""#;
        let config: TestConfig = toml::from_str(toml).expect("failed to parse");
        assert_eq!(
            config.when,
            Condition::Not(Box::new(Condition::Mode(ModeCondition::Follow)))
        );
    }

    #[test]
    fn condition_error_display() {
        let err = ConditionError::UnknownPrefix("arch".to_string());
        assert!(err.to_string().contains("arch"));
        assert!(err.to_string().contains("expected"));

        let err = ConditionError::MissingPrefix("macos".to_string());
        assert!(err.to_string().contains("macos"));

        let err = ConditionError::UnknownOs("freebsd".to_string());
        assert!(err.to_string().contains("freebsd"));

        let err = ConditionError::UnknownMode("stream".to_string());
        assert!(err.to_string().contains("stream"));
    }
}
