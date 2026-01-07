// third-party imports
use serde::{Deserialize, Serialize};

// ---

/// Semantic style role for theme inheritance (v1 feature).
///
/// Defines reusable styles (e.g., primary, warning) that elements can inherit from.
/// Roles are serialized in kebab-case (e.g., `AccentSecondary` -> `accent-secondary`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    #[default]
    Default,
    Primary,
    Secondary,
    Strong,
    Muted,
    Accent,
    AccentSecondary,
    Message,
    Key,
    Value,
    Syntax,
    Status,
    Level,
    Unknown,
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_plain::to_string(self) {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "{:?}", self),
        }
    }
}
