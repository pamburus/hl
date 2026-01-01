// std imports
use std::{io, path::Path, str, sync::Arc};

// third-party imports
use thiserror::Error;
use yaml_peg::serde as yaml;

// local imports
use crate::xerr::{Highlight, HighlightQuoted, Suggestions};

// relative imports
pub use super::{Role, StyleBase, ThemeInfo, Version};

/// Top-level error type for theme operations.
///
/// Wraps lower-level errors with context about which theme operation failed.
#[derive(Error, Debug)]
pub enum Error {
    /// Theme file not found (neither custom nor embedded).
    #[error("theme {name} not found", name=.name.hlq())]
    ThemeNotFound { name: Arc<str>, suggestions: Suggestions },

    /// Theme overlay file not found (neither custom nor embedded).
    #[error("theme overlay {name} not found", name=.name.hlq())]
    ThemeOverlayNotFound { name: Arc<str>, suggestions: Suggestions },

    /// Unsupported file type for theme.
    #[error("failed to load theme {path}: unsupported file type {extension}", path=.path.hlq(), extension=.extension.hlq())]
    UnsupportedFileType { path: Arc<str>, extension: Arc<str> },

    /// Failed to load an embedded theme.
    #[error("failed to load theme {name}: {source}", name=.name.hlq())]
    FailedToLoadEmbeddedTheme { name: Arc<str>, source: ThemeLoadError },

    /// Failed to load a custom theme from the filesystem.
    #[error("failed to load theme {name} from {path}: {source}", name=.name.hlq(), path=.path.hlq())]
    FailedToLoadCustomTheme {
        name: Arc<str>,
        path: Arc<Path>,
        source: ThemeLoadError,
    },

    /// Failed to list custom themes directory.
    #[error("failed to list custom themes: {0}")]
    FailedToListCustomThemes(#[from] io::Error),

    /// Invalid theme tag value.
    #[error("invalid tag {value}", value=.value.hlq())]
    InvalidTag { value: Arc<str>, suggestions: Suggestions },

    /// Failed to resolve theme styles (e.g., circular inheritance).
    #[error("failed to resolve theme {name}: {source}", name=.info.name.hlq())]
    FailedToResolveTheme {
        info: Arc<ThemeInfo>,
        source: StyleResolveError,
    },

    /// Invalid theme version format.
    #[error("invalid version format: {format}", format=.0.hlq())]
    InvalidVersion(Arc<str>),
}

/// Theme loading and resolution errors.
///
/// Occurs during theme parsing or style resolution.
#[derive(Error, Debug)]
pub enum ThemeLoadError {
    /// External I/O or parsing error.
    #[error(transparent)]
    External(#[from] ExternalError),

    /// Theme version is not supported (e.g., v2.0 when max is v1.0).
    #[error("theme version {requested} is not supported (latest is {latest})", requested=.requested.hl(), latest=.latest.hl())]
    UnsupportedVersion {
        requested: Version,
        nearest: Version,
        latest: Version,
    },

    #[error(transparent)]
    ResolveError(#[from] StyleResolveError),
}

/// Style inventory resolution error.
///
/// Occurs during style role-based inventory resolution.
#[derive(Error, Debug)]
pub enum StyleResolveError {
    /// Style recursion limit exceeded (circular inheritance or too deep).
    ///
    /// Limits role inheritance to 64 levels to prevent infinite loops (FR-046, FR-047).
    #[error("style inheritance depth exceeded limit {limit} for role {role} with base {base}", limit=.limit.hl(), role=.role.hlq(), base=.base.hlq())]
    RecursionLimitExceeded { role: Role, base: StyleBase, limit: usize },
}

/// External errors from I/O and parsing operations.
///
/// These are low-level errors that occur when reading files or parsing
/// theme file formats (YAML, TOML, JSON).
#[derive(Error, Debug)]
pub enum ExternalError {
    /// I/O error (file not found, permission denied, etc.).
    #[error(transparent)]
    Io(#[from] io::Error),

    /// YAML parsing error.
    #[error("{source}", source=.0.msg)]
    YamlSerdeError(#[from] yaml::SerdeError),

    /// TOML parsing error.
    #[error(transparent)]
    TomlError(#[from] toml::de::Error),

    /// JSON parsing error.
    #[error("failed to parse json: {0}")]
    JsonError(#[from] serde_json::Error),

    /// UTF-8 decoding error.
    #[error("failed to parse utf-8: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
