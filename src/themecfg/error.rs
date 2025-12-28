// std imports
use std::{io, path::Path, str, sync::Arc};

// third-party imports
use thiserror::Error;
use yaml_peg::serde as yaml;

// local imports
use crate::xerr::{Highlight, HighlightQuoted, Suggestions};

// relative imports
pub use super::{Role, StyleBase, ThemeInfo, ThemeVersion};

/// Top-level error type for theme operations.
///
/// This error type wraps lower-level errors ([`ThemeLoadError`], [`ExternalError`])
/// with context about which theme operation failed.
///
/// # Error Hierarchy
///
/// - [`enum@Error`] (this type) - High-level theme operations
///   - [`ThemeLoadError`] - Theme loading/resolution errors
///     - [`ExternalError`] - I/O and parsing errors
///
/// # Error Context
///
/// All error variants include rich context:
/// - `ThemeNotFound`: includes theme name and suggestions for similar names
/// - `FailedToLoadEmbeddedTheme`: includes theme name and nested error
/// - `FailedToLoadCustomTheme`: includes theme name, file path, and nested error
/// - `FailedToResolveTheme`: includes full `ThemeInfo` (name, source, origin) and nested error
///
/// Nested errors (`ThemeLoadError`) may include:
/// - Parse errors with line/column information
/// - Version incompatibility details
/// - Style recursion errors with the problematic role name
#[derive(Error, Debug)]
pub enum Error {
    /// Theme file not found (neither custom nor embedded).
    ///
    /// Includes suggestions for similar theme names to help users correct typos.
    #[error("theme {name} not found", name=.name.hlq())]
    ThemeNotFound { name: Arc<str>, suggestions: Suggestions },

    /// Unsupported file type for theme.
    #[error("failed to load theme {path}: unsupported file type {extension}", path=.path.hlq(), extension=.extension.hlq())]
    UnsupportedFileType { path: Arc<str>, extension: Arc<str> },

    /// Failed to load an embedded theme.
    ///
    /// This wraps errors that occur when loading themes built into the binary.
    #[error("failed to load theme {name}: {source}", name=.name.hlq())]
    FailedToLoadEmbeddedTheme { name: Arc<str>, source: ThemeLoadError },

    /// Failed to load a custom theme from the filesystem.
    ///
    /// This wraps errors that occur when loading user-provided theme files,
    /// including the file path for better debugging.
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
    ///
    /// Includes suggestions for valid tag names.
    #[error("invalid tag {value}", value=.value.hlq())]
    InvalidTag { value: Arc<str>, suggestions: Suggestions },

    /// Failed to resolve theme styles.
    ///
    /// This occurs after the theme file is loaded successfully but style
    /// resolution fails (e.g., circular role inheritance).
    #[error("failed to resolve theme {name}: {source}", name=.info.name.hlq())]
    FailedToResolveTheme {
        info: Arc<ThemeInfo>,
        source: ThemeLoadError,
    },

    /// Invalid theme version format.
    #[error("invalid version format: {format}", format=.0.hlq())]
    InvalidVersion(Arc<str>),
}

/// Theme loading and resolution errors.
///
/// These errors occur during theme file parsing and style resolution.
/// They are typically wrapped by [`enum@Error`] variants that add context
/// about which theme failed.
///
/// # Examples
///
/// ```text
/// UnsupportedVersion: theme version 2.0 is not supported (maximum supported: 1.0)
/// StyleRecursionLimitExceeded: style recursion limit exceeded while resolving role primary
/// External(YamlSerdeError): failed to parse yaml: unknown field `typo`
/// ```
#[derive(Error, Debug)]
pub enum ThemeLoadError {
    /// External I/O or parsing error.
    ///
    /// Wraps errors from file I/O, YAML/TOML/JSON parsing, etc.
    #[error(transparent)]
    External(#[from] ExternalError),

    /// Theme version is not supported.
    ///
    /// This occurs when a theme file specifies a version newer than the
    /// current implementation supports (e.g., loading a v2.0 theme when
    /// only v1.0 is supported).
    ///
    /// # Example Error Message
    ///
    /// ```text
    /// theme version 2.0 is not supported (maximum supported: 1.0)
    /// ```
    #[error("theme version {requested} is not supported", requested=.requested.hl())]
    UnsupportedVersion {
        requested: ThemeVersion,
        supported: ThemeVersion,
    },

    /// Style recursion limit exceeded during role resolution.
    ///
    /// This occurs when there is circular inheritance in role-based styles
    /// (e.g., role A inherits from role B, which inherits from role A) or
    /// when inheritance chains exceed the maximum depth of 64 levels.
    ///
    /// The recursion limit prevents infinite loops and stack overflow.
    ///
    /// # Specification Requirements
    ///
    /// - **FR-046**: V1 role-to-role inheritance via the `style` field MUST support
    ///   a maximum depth of 64 levels
    /// - **FR-047**: V1 themes MUST detect circular role references and exit with error
    ///
    /// # Example Error Message
    ///
    /// ```text
    /// style recursion limit exceeded while resolving role primary
    /// ```
    ///
    /// # Common Causes
    ///
    /// **Circular inheritance** in theme file:
    /// ```yaml
    /// styles:
    ///   primary:
    ///     style: [secondary]
    ///   secondary:
    ///     style: [primary]  # Circular!
    /// ```
    ///
    /// **Excessively deep chain** (rare):
    /// ```yaml
    /// styles:
    ///   role1: { style: [role2] }
    ///   role2: { style: [role3] }
    ///   # ... 65+ levels deep
    /// ```
    #[error("style inheritance depth exceeded limit {limit} for role {role} with base {base}", limit=.limit.hl(), role=.role.hlq(), base=.base.hlq())]
    StyleRecursionLimitExceeded { role: Role, base: StyleBase, limit: usize },
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
    #[error("failed to parse yaml: {0}")]
    YamlSerdeError(#[from] yaml::SerdeError),

    /// TOML parsing error.
    #[error(transparent)]
    TomlError(#[from] toml::de::Error),

    /// JSON parsing error.
    #[error("failed to parse json: {0}")]
    JsonError(#[from] serde_json::Error),

    /// UTF-8 decoding error.
    #[error("failed to parse utf-8 string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
