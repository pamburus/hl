//! Theme configuration system for `hl`.
//!
//! This module provides theme loading, parsing, and resolution functionality.
//! It supports two theme format versions:
//!
//! - **v0** (legacy): Simple, flat, element-based styling
//! - **v1** (current): Semantic roles, base styles, mode diffs, and advanced features
//!
//! # Quick Start
//!
//! ```no_run
//! use hl::themecfg::Theme;
//! use hl::app::AppDirs;
//!
//! let app_dirs = AppDirs::default();
//! let theme = Theme::load(&app_dirs, "monokai")?;
//! # Ok::<(), hl::themecfg::Error>(())
//! ```
//!
//! # Theme Formats
//!
//! ## V0 Format (Legacy)
//!
//! V0 themes use simple element-based styling with no semantic roles:
//!
//! ```yaml
//! version: "0.0"
//! elements:
//!   level:
//!     foreground: "#00ff00"
//!     modes: [bold]
//! ```
//!
//! Features:
//! - Lenient parsing (ignores unknown fields for forward compatibility)
//! - Direct element → style mapping
//! - Simple mode lists (no diff syntax)
//!
//! ## V1 Format (Current)
//!
//! V1 themes support semantic roles and style inheritance:
//!
//! ```yaml
//! version: "1.0"
//! styles:
//!   primary:
//!     foreground: "#00ff00"
//!     modes: [bold]
//!   secondary:
//!     style: [primary]  # Inherit from primary
//!     modes: [+italic]  # Add italic to inherited modes
//! elements:
//!   level:
//!     style: [secondary]  # Reference role-based style
//! ```
//!
//! Features:
//! - Strict parsing (fails on unknown fields)
//! - Role-based styles with inheritance
//! - Mode diff syntax (`+mode`, `-mode`)
//! - `$schema` field support for IDE validation
//!
//! # Loading Pipeline
//!
//! 1. **Load**: Theme file is loaded from custom directory or embedded themes
//! 2. **Parse**: YAML/TOML/JSON is deserialized based on detected version
//! 3. **Convert**: V0 themes are converted to V1 format
//! 4. **Merge**: Theme is merged with `@default` theme
//! 5. **Resolve**: Role-based styles are resolved to concrete element styles
//!
//! # Public API Types
//!
//! - [`Theme`]: Fully resolved theme (output of loading pipeline)
//! - [`RawTheme`]: Unresolved theme (before resolution, allows modifications)
//! - [`Style`]: Resolved style with concrete foreground/background/modes
//! - [`RawStyle`]: Unresolved style (may reference roles)
//! - [`Element`]: Theme element enum (level, timestamp, etc.)
//! - [`Role`]: Semantic style role (primary, secondary, warning, etc.)
//!
//! # Advanced Usage
//!
//! For advanced customization, use [`Theme::load_raw()`] to get an unresolved theme,
//! modify it, then call [`RawTheme::resolve()`]:
//!
//! ```no_run
//! use hl::themecfg::{Theme, RawTheme};
//! use hl::app::AppDirs;
//!
//! let app_dirs = AppDirs::default();
//! let mut raw_theme = Theme::load_raw(&app_dirs, "monokai")?;
//!
//! // Customize the theme
//! // raw_theme.styles.0.insert(...);
//!
//! let theme = raw_theme.resolve()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Error Handling
//!
//! All errors include context about what failed:
//!
//! - **Theme not found**: Includes suggestions for similar theme names
//! - **Parse errors**: Includes file path and line/column information
//! - **Version errors**: Shows requested vs. supported version
//! - **Resolution errors**: Shows theme name and problematic role (for circular inheritance)
//!
//! Example error messages:
//! ```text
//! failed to load theme "my-theme" from "path/to/my-theme.yaml": unknown field `invalid`
//! failed to resolve theme "my-theme": style recursion limit exceeded while resolving role primary
//! ```

// std imports
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::{self, Write},
    hash::Hash,
    io::{self, ErrorKind},
    ops::{Add, AddAssign},
    path::{Component, Path, PathBuf},
    str::{self, FromStr},
    sync::Arc,
};

// third-party imports
use derive_more::Deref;
use enum_map::Enum;
use enumset::{EnumSet, EnumSetType};
use rust_embed::RustEmbed;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};
use serde_json as json;
use serde_value::Value;
use strum::{Display, EnumIter, IntoEnumIterator};
use thiserror::Error;
use yaml_peg::serde as yaml;

// local imports
use crate::{
    appdirs::AppDirs,
    level::Level,
    xerr::{Highlight, HighlightQuoted, Suggestions},
};

// Version-specific modules
pub mod v0;
pub mod v1;

// Re-export v1 types that are part of the public API
// (Element comes from v0, re-exported by v1)
pub use v1::{Element, Role, StyleBase};

// Type aliases for the public API
/// An unresolved theme, before style resolution.
///
/// This is a type alias for [`v1::Theme`], which contains unresolved [`RawStyle`]
/// definitions that may reference role-based styles.
///
/// Use [`RawTheme::resolve()`](v1::Theme::resolve) to convert to a fully resolved [`Theme`].
///
/// # Examples
///
/// ```no_run
/// use hl::themecfg::{Theme, RawTheme};
/// use hl::app::AppDirs;
///
/// let app_dirs = AppDirs::default();
/// let raw: RawTheme = Theme::load_raw(&app_dirs, "monokai")?;
/// let resolved: Theme = raw.resolve()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub type RawTheme = v1::Theme;

/// An unresolved style, before role resolution.
///
/// This is a type alias for [`v1::Style`], which may contain:
/// - Base style references (inheriting from roles)
/// - Mode diffs (additions/removals using `+`/`-` prefix)
///
/// After resolution, this becomes a concrete [`Style`] with all values computed.
pub type RawStyle = v1::Style;

// Private constants
const DEFAULT_THEME_NAME: &str = "@default";

// ---

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
/// # Examples
///
/// ```no_run
/// use hl::themecfg::{Theme, Error};
/// use hl::app::AppDirs;
///
/// let app_dirs = AppDirs::default();
/// match Theme::load(&app_dirs, "invalid-theme") {
///     Ok(theme) => println!("Loaded theme successfully"),
///     Err(Error::ThemeNotFound { name, suggestions }) => {
///         eprintln!("Theme '{}' not found", name);
///         if !suggestions.is_empty() {
///             eprintln!("Did you mean: {}", suggestions);
///         }
///     }
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
#[derive(Error, Debug)]
pub enum Error {
    /// Theme file not found (neither custom nor embedded).
    ///
    /// Includes suggestions for similar theme names to help users correct typos.
    #[error("unknown theme {name}", name=.name.hlq())]
    ThemeNotFound { name: Arc<str>, suggestions: Suggestions },

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
    #[error("failed to resolve theme {name}: {source}", name=.name.hlq())]
    FailedToResolveTheme { name: Arc<str>, source: ThemeLoadError },

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
    /// (e.g., role A inherits from role B, which inherits from role A).
    ///
    /// The recursion limit is set to prevent infinite loops and stack overflow.
    ///
    /// # Example Error Message
    ///
    /// ```text
    /// style recursion limit exceeded while resolving role primary
    /// ```
    ///
    /// # Common Cause
    ///
    /// Circular inheritance in theme file:
    /// ```yaml
    /// styles:
    ///   primary:
    ///     style: [secondary]
    ///   secondary:
    ///     style: [primary]  # Circular!
    /// ```
    #[error("style recursion limit exceeded while resolving role {role}", role=.role.hlq())]
    StyleRecursionLimitExceeded { role: Role },
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

// ---

// Role is now defined in v1 module and re-exported above

// ---
// RawTheme is a type alias to v1::RawTheme
// All merge/resolve logic is in v1
// Loading helpers are defined below as Theme static methods

// ---

/// A fully resolved theme with all styles resolved and ready for use.
///
/// This type contains element-based styles that have been fully resolved
/// from role-based styles, with all inheritance and merging applied.
/// It is the output of [`RawTheme::resolve()`] and the input for creating
/// a runtime [`crate::theme::Theme`].
///
/// Unlike [`RawTheme`], which contains unresolved [`RawStyle`] references that may
/// use role-based inheritance, `Theme` contains only [`Style`]
/// instances with concrete foreground, background, and mode values.
#[derive(Debug, Default)]
pub struct Theme {
    pub tags: EnumSet<Tag>,
    pub version: ThemeVersion,
    pub elements: StylePack<Element, Style>,
    pub levels: HashMap<Level, StylePack<Element, Style>>,
    pub indicators: IndicatorPack<Style>,
}

impl Theme {
    /// Load a fully resolved theme by name.
    ///
    /// This is the primary method for loading themes. It performs the complete
    /// theme loading pipeline:
    /// 1. Loads the theme from custom directory or embedded themes
    /// 2. Merges with the `@default` theme
    /// 3. Resolves all role-based styles to concrete styles
    ///
    /// The theme is searched in the following order:
    /// - Custom themes in `{config_dir}/themes/`
    /// - Embedded themes (built into the binary)
    ///
    /// All themes are automatically merged with `@default` to ensure all
    /// required elements have styles defined.
    ///
    /// # Arguments
    ///
    /// * `app_dirs` - Application directories configuration
    /// * `name` - Theme name (without file extension)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use hl::themecfg::Theme;
    /// use hl::app::AppDirs;
    ///
    /// let app_dirs = AppDirs::default();
    /// let theme = Theme::load(&app_dirs, "monokai")?;
    /// # Ok::<(), hl::themecfg::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Theme file cannot be found (neither custom nor embedded)
    /// - Theme file cannot be parsed (invalid YAML/TOML/JSON)
    /// - Theme version is unsupported (e.g., future version)
    /// - Style resolution fails (e.g., circular role inheritance)
    ///
    /// Error messages include context:
    /// - Theme name
    /// - File path (for custom themes)
    /// - Specific error details (parse error, unsupported version, recursion, etc.)
    pub fn load(app_dirs: &AppDirs, name: &str) -> Result<Self> {
        Self::load_raw(app_dirs, name)?
            .resolve()
            .map_err(|source| Error::FailedToResolveTheme {
                name: name.into(),
                source,
            })
    }

    /// Load an unresolved (raw) theme by name.
    ///
    /// This method loads and merges the theme but **does not resolve styles**.
    /// It returns a [`RawTheme`] which contains unresolved [`RawStyle`] definitions
    /// that may reference role-based styles.
    ///
    /// This is useful for advanced use cases where you want to:
    /// - Inspect the theme structure before resolution
    /// - Apply custom modifications to the theme
    /// - Merge multiple themes programmatically
    /// - Defer resolution until later
    ///
    /// After modifications, call [`RawTheme::resolve()`] to get a fully resolved [`Theme`].
    ///
    /// # Arguments
    ///
    /// * `app_dirs` - Application directories configuration
    /// * `name` - Theme name (without file extension)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use hl::themecfg::{Theme, RawTheme};
    /// use hl::app::AppDirs;
    ///
    /// let app_dirs = AppDirs::default();
    ///
    /// // Load raw theme
    /// let mut raw_theme = Theme::load_raw(&app_dirs, "monokai")?;
    ///
    /// // Apply custom modifications
    /// // raw_theme.styles.0.insert(...);
    ///
    /// // Resolve to get final theme
    /// let theme = raw_theme.resolve()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Theme file cannot be found
    /// - Theme file cannot be parsed
    /// - Theme version is unsupported
    ///
    /// Note: Style resolution errors (e.g., circular inheritance) will only
    /// occur when calling [`RawTheme::resolve()`], not during `load_raw()`.
    pub fn load_raw(app_dirs: &AppDirs, name: &str) -> Result<RawTheme> {
        let theme = Self::load_embedded::<Assets>(DEFAULT_THEME_NAME)?;

        Ok(theme.merged(match Self::load_from(&Self::themes_dir(app_dirs), name) {
            Ok(v) => Ok(v),
            Err(Error::ThemeNotFound { .. }) => match Self::load_embedded::<Assets>(name) {
                Ok(v) => Ok(v),
                Err(Error::ThemeNotFound { name, mut suggestions }) => {
                    if let Ok(variants) = Self::custom_names(app_dirs) {
                        let variants = variants.into_iter().filter_map(|v| v.ok());
                        suggestions = suggestions.merge(Suggestions::new(&name, variants));
                    }
                    Err(Error::ThemeNotFound { name, suggestions })
                }
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }?))
    }

    pub fn embedded(name: &str) -> Result<Self> {
        Self::load_embedded::<Assets>(name)?
            .resolve()
            .map_err(|source| Error::FailedToResolveTheme {
                name: name.into(),
                source,
            })
    }

    pub fn list(app_dirs: &AppDirs) -> Result<HashMap<Arc<str>, ThemeInfo>> {
        let mut result = HashMap::new();

        for name in Self::embedded_names() {
            result.insert(name, ThemeOrigin::Stock.into());
        }

        if let Ok(names) = Self::custom_names(app_dirs) {
            for name in names {
                match name {
                    Ok(name) => {
                        result.insert(name, ThemeOrigin::Custom.into());
                    }
                    Err(e) => {
                        eprintln!("failed to list custom themes: {}", e);
                    }
                }
            }
        }

        Ok(result)
    }

    fn load_embedded<S: RustEmbed>(name: &str) -> Result<RawTheme> {
        for format in Format::iter() {
            let filename = Self::filename(name, format);
            if let Some(file) = S::get(&filename) {
                return Self::from_buf(file.data.as_ref(), format).map_err(|e| Error::FailedToLoadEmbeddedTheme {
                    name: name.into(),
                    source: e,
                });
            }
        }

        let suggestions = Suggestions::new(name, Self::embedded_names());

        Err(Error::ThemeNotFound {
            name: name.into(),
            suggestions,
        })
    }

    fn deserialize<T>(s: &str, format: Format) -> Result<T, ExternalError>
    where
        T: for<'de> Deserialize<'de>,
    {
        match format {
            Format::Yaml => Ok(yaml::from_str(s)?.remove(0)),
            Format::Toml => Ok(toml::from_str(s)?),
            Format::Json => Ok(json::from_str(s)?),
        }
    }

    fn from_buf(data: &[u8], format: Format) -> Result<RawTheme, ThemeLoadError> {
        let s = std::str::from_utf8(data).map_err(ExternalError::from)?;

        // Peek at version to decide which deserialization path to use
        let version = Self::peek_version(s, format)?;

        if version.major == 0 {
            // Validate v0 version before deserializing
            v0::Theme::validate_version(&version)?;
            // V0 themes use lenient deserialization (ignore unknown fields/variants)
            let theme: v0::Theme = Self::deserialize(s, format)?;
            // Convert v0 to v1
            Ok(theme.into())
        } else {
            // Validate v1 version before deserializing
            RawTheme::validate_version(&version)?;
            // V1+ themes use strict deserialization
            let theme: RawTheme = Self::deserialize(s, format)?;
            Ok(theme)
        }
    }

    fn peek_version(s: &str, format: Format) -> Result<ThemeVersion, ExternalError> {
        #[derive(Deserialize)]
        struct VersionOnly {
            #[serde(default)]
            version: ThemeVersion,
        }

        let data: VersionOnly = Self::deserialize(s, format)?;
        Ok(data.version)
    }

    fn load_from(dir: &Path, name: &str) -> Result<RawTheme> {
        for format in Format::iter() {
            let filename = Self::filename(name, format);
            let path = PathBuf::from(&filename);
            let path = if matches!(path.components().next(), Some(Component::ParentDir | Component::CurDir)) {
                path
            } else {
                dir.join(&filename)
            };

            let map_err = |e: ThemeLoadError, path: &Path| Error::FailedToLoadCustomTheme {
                name: name.into(),
                path: path.into(),
                source: e,
            };

            match std::fs::read(&path) {
                Ok(data) => {
                    let theme = Self::from_buf(&data, format).map_err(|e| map_err(e, &path))?;
                    return Ok(theme);
                }
                Err(e) => match e.kind() {
                    ErrorKind::NotFound => continue,
                    _ => return Err(map_err(ExternalError::from(e).into(), &path)),
                },
            }
        }

        Err(Error::ThemeNotFound {
            name: name.into(),
            suggestions: Suggestions::none(),
        })
    }

    fn filename(name: &str, format: Format) -> String {
        if Self::strip_extension(name, format).is_some() {
            return name.to_string();
        }

        format!("{}.{}", name, format.extension())
    }

    fn themes_dir(app_dirs: &AppDirs) -> PathBuf {
        app_dirs.config_dir.join("themes")
    }

    fn embedded_names() -> impl IntoIterator<Item = Arc<str>> {
        Assets::iter().filter_map(|a| {
            Self::strip_known_extension(&a)
                .filter(|&n| n != DEFAULT_THEME_NAME)
                .map(|n| n.into())
        })
    }

    fn custom_names(app_dirs: &AppDirs) -> Result<impl IntoIterator<Item = Result<Arc<str>>> + use<>> {
        let path = Self::themes_dir(app_dirs);
        let dir = Path::new(&path);
        Ok(dir
            .read_dir()?
            .map(|item| {
                let item = item?;
                if !item.file_type()?.is_file() {
                    return Ok(None);
                }
                Ok(item
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|a| Self::strip_known_extension(a).map(|n| n.into())))
            })
            .filter_map(|x| x.transpose()))
    }

    fn strip_extension(filename: &str, format: Format) -> Option<&str> {
        filename
            .strip_suffix(format.extension())
            .and_then(|r| r.strip_suffix("."))
    }

    fn strip_known_extension(filename: &str) -> Option<&str> {
        for format in Format::iter() {
            if let Some(name) = Self::strip_extension(filename, format) {
                return Some(name);
            }
        }
        None
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, EnumIter)]
pub enum Format {
    Yaml,
    Toml,
    Json,
}

impl Format {
    pub fn extension(&self) -> &str {
        match self {
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Json => "json",
        }
    }
}

// ---

#[derive(Debug, Ord, PartialOrd, Hash, Enum, Deserialize, EnumSetType, Display)]
#[strum(serialize_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum Tag {
    Dark,
    Light,
    #[strum(serialize = "16color")]
    #[serde(rename = "16color")]
    Palette16,
    #[strum(serialize = "256color")]
    #[serde(rename = "256color")]
    Palette256,
    #[strum(serialize = "truecolor")]
    #[serde(rename = "truecolor")]
    TrueColor,
}

impl FromStr for Tag {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_plain::from_str(s).map_err(|_| Error::InvalidTag {
            value: s.into(),
            suggestions: Suggestions::new(s, EnumSet::<Tag>::all().iter().map(|v| v.to_string())),
        })
    }
}

// ---

/// Source location for a theme, used for error reporting.
///
/// This enum tracks where a theme was loaded from, enabling better
/// error messages when theme resolution or validation fails.
#[derive(Debug, Clone)]
pub enum ThemeSource {
    /// Theme loaded from an embedded resource
    Embedded { name: Arc<str> },
    /// Theme loaded from a file on disk
    File { path: Arc<Path> },
    /// Theme loaded from an in-memory buffer
    Buffer,
}

impl fmt::Display for ThemeSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Embedded { name } => write!(f, "embedded theme '{}'", name),
            Self::File { path } => write!(f, "theme file '{}'", path.display()),
            Self::Buffer => write!(f, "in-memory theme"),
        }
    }
}

// ---

#[derive(Debug, Clone)]
pub struct ThemeInfo {
    pub origin: ThemeOrigin,
}

impl From<ThemeOrigin> for ThemeInfo {
    fn from(origin: ThemeOrigin) -> Self {
        Self { origin }
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum ThemeOrigin {
    Stock,
    Custom,
}

// ---

pub type StyleInventory = StylePack<Role, Style>;

#[derive(Clone, Debug, Deref)]
pub struct StylePack<K = Element, S = Style>(HashMap<K, S>);

impl<K, S> Default for StylePack<K, S> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<K, S> StylePack<K, S>
where
    K: Eq + Hash,
{
    pub fn new(items: HashMap<K, S>) -> Self {
        Self(items)
    }

    pub fn items(&self) -> &HashMap<K, S> {
        &self.0
    }
}

impl<S> StylePack<Role, S> {
    pub fn merge(&mut self, patch: Self) {
        self.0.extend(patch.0);
    }

    pub fn merged(mut self, patch: Self) -> Self {
        self.merge(patch);
        self
    }
}

impl<S> StylePack<Element, S> {
    pub fn merge(&mut self, patch: Self, flags: MergeFlags)
    where
        S: Clone + for<'a> Merge<&'a S>,
    {
        if flags.contains(MergeFlag::ReplaceGroups) {
            for (parent, child) in Element::pairs() {
                if patch.contains_key(child) {
                    self.0.remove(parent);
                }
            }
        }

        if flags.contains(MergeFlag::ReplaceElements) {
            self.0.extend(patch.0);
            return;
        }

        for (key, patch) in patch.0 {
            self.0
                .entry(key)
                .and_modify(|v| *v = v.clone().merged(&patch, flags))
                .or_insert(patch);
        }
    }

    pub fn merged(mut self, patch: Self, flags: MergeFlags) -> Self
    where
        S: Clone + for<'a> Merge<&'a S>,
    {
        self.merge(patch, flags);
        self
    }
}

impl Merge<&StylePack> for StylePack {
    fn merge(&mut self, other: &StylePack<Element, Style>, flags: MergeFlags) {
        Self::merge(self, other.clone(), flags);
    }
}

// ---

#[derive(Debug, Hash, Ord, PartialOrd, EnumSetType, Deserialize)]
pub enum MergeFlag {
    ReplaceElements,
    ReplaceGroups,
    ReplaceModes,
}

pub type MergeFlags = EnumSet<MergeFlag>;

// ---

// StylePack::resolve is now in v1 module only

impl<K, S, I: Into<HashMap<K, S>>> From<I> for StylePack<K, S> {
    fn from(i: I) -> Self {
        Self(i.into())
    }
}

impl<'de, K, S> Deserialize<'de> for StylePack<K, S>
where
    K: Deserialize<'de> + Eq + Hash,
    S: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(StylePackDeserializeVisitor::<K, S>::new())
    }
}

// ---

struct StylePackDeserializeVisitor<K, S> {
    _phantom: std::marker::PhantomData<(K, S)>,
}

impl<K, S> StylePackDeserializeVisitor<K, S> {
    #[inline]
    fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'de, K, S> Visitor<'de> for StylePackDeserializeVisitor<K, S>
where
    K: Deserialize<'de> + Eq + Hash,
    S: Deserialize<'de>,
{
    type Value = StylePack<K, S>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("style pack object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> std::result::Result<Self::Value, A::Error> {
        let mut items = HashMap::new();

        // Use Value as a generic "any value" type to handle unknown keys.
        // This is format-agnostic and works with all serde formats (YAML, TOML, JSON).
        // This allows us to:
        // 1. Deserialize the key as Value
        // 2. Try to convert it to K (the expected key type)
        // 3. If conversion fails (unknown key), discard the value
        // This provides forward compatibility by silently ignoring unknown elements.
        while let Some(key) = access.next_key::<Value>()? {
            if let Ok(key) = K::deserialize(key) {
                let value: S = access.next_value()?;
                items.insert(key, value);
            } else {
                _ = access.next_value::<Value>()?;
            }
        }

        Ok(StylePack(items))
    }
}

// ---

// Element is now defined in v0 module and re-exported above (via v1)

// ---

// StyleBase is now defined in v1 module and re-exported above

// ---

// Style (unresolved) is now defined in v1 module and re-exported above

// ---

// StyleResolver is now defined in v1 module only

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct Style {
    pub modes: ModeSetDiff,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            modes: ModeSetDiff::new(),
            foreground: None,
            background: None,
        }
    }

    pub fn modes(self, modes: ModeSetDiff) -> Self {
        Self { modes, ..self }
    }

    pub fn foreground(self, foreground: Option<Color>) -> Self {
        Self { foreground, ..self }
    }

    pub fn background(self, background: Option<Color>) -> Self {
        Self { background, ..self }
    }
}

impl Merge<&Style> for Style {
    fn merge(&mut self, other: &Style, flags: MergeFlags) {
        if flags.contains(MergeFlag::ReplaceModes) {
            self.modes = other.modes;
        } else {
            self.modes += other.modes;
        }
        if let Some(color) = other.foreground {
            self.foreground = Some(color);
        }
        if let Some(color) = other.background {
            self.background = Some(color);
        }
    }
}

impl Merge<&RawStyle> for Style {
    fn merge(&mut self, other: &RawStyle, flags: MergeFlags) {
        if flags.contains(MergeFlag::ReplaceModes) {
            self.modes = other.modes;
        } else {
            self.modes += other.modes;
        }
        if let Some(color) = other.foreground {
            self.foreground = Some(color);
        }
        if let Some(color) = other.background {
            self.background = Some(color);
        }
    }
}

// ---

#[derive(Debug, Deserialize, Serialize, EnumSetType)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Bold,
    Faint,
    Italic,
    Underline,
    SlowBlink,
    RapidBlink,
    Reverse,
    Conceal,
    CrossedOut,
}

pub type ModeSet = EnumSet<Mode>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ModeSetDiff {
    pub adds: ModeSet,
    pub removes: ModeSet,
}

impl ModeSetDiff {
    pub const fn new() -> Self {
        Self {
            adds: ModeSet::new(),
            removes: ModeSet::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.adds.is_empty() && self.removes.is_empty()
    }
}

impl Add<ModeSetDiff> for ModeSetDiff {
    type Output = ModeSetDiff;

    fn add(mut self, rhs: ModeSetDiff) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<ModeSetDiff> for ModeSetDiff {
    fn add_assign(&mut self, rhs: ModeSetDiff) {
        let adds = (self.adds | rhs.adds) - rhs.removes;
        let removes = (self.removes | rhs.removes) - rhs.adds;

        self.adds = adds;
        self.removes = removes;
    }
}

impl From<ModeSet> for ModeSetDiff {
    fn from(modes: ModeSet) -> Self {
        Self {
            adds: modes,
            removes: ModeSet::new(),
        }
    }
}

impl From<Mode> for ModeSetDiff {
    fn from(mode: Mode) -> Self {
        Self {
            adds: mode.into(),
            removes: ModeSet::new(),
        }
    }
}

impl<'de> Deserialize<'de> for ModeSetDiff {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let diffs = Vec::<ModeDiff>::deserialize(deserializer)?;
        let mut result = ModeSetDiff::new();

        for diff in diffs {
            match diff.action {
                ModeDiffAction::Add => result.adds.insert(diff.mode),
                ModeDiffAction::Remove => result.removes.insert(diff.mode),
            };
        }

        Ok(result)
    }
}

impl Serialize for ModeSetDiff {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut diffs = Vec::new();

        for mode in self.adds.iter() {
            diffs.push(ModeDiff::add(mode));
        }

        for mode in self.removes.iter() {
            diffs.push(ModeDiff::remove(mode));
        }

        diffs.serialize(serializer)
    }
}

// ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeDiff {
    pub action: ModeDiffAction,
    pub mode: Mode,
}

impl ModeDiff {
    pub fn add(mode: Mode) -> Self {
        Self {
            action: ModeDiffAction::Add,
            mode,
        }
    }

    pub fn remove(mode: Mode) -> Self {
        Self {
            action: ModeDiffAction::Remove,
            mode,
        }
    }
}

impl<'de> Deserialize<'de> for ModeDiff {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if let Some(s) = s.strip_prefix('+') {
            let mode: Mode = serde_plain::from_str(s).map_err(serde::de::Error::custom)?;
            Ok(ModeDiff::add(mode))
        } else if let Some(s) = s.strip_prefix('-') {
            let mode: Mode = serde_plain::from_str(s).map_err(serde::de::Error::custom)?;
            Ok(ModeDiff::remove(mode))
        } else {
            let mode: Mode = serde_plain::from_str(&s).map_err(serde::de::Error::custom)?;
            Ok(ModeDiff::add(mode))
        }
    }
}

impl Serialize for ModeDiff {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let prefix = match self.action {
            ModeDiffAction::Add => "+",
            ModeDiffAction::Remove => "-",
        };
        let mode_str = serde_plain::to_string(&self.mode).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&format!("{}{}", prefix, mode_str))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeDiffAction {
    Add,
    Remove,
}

// ---

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum Color {
    Plain(PlainColor),
    Palette(u8),
    RGB(RGB),
}

// ---

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlainColor {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

// ---

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
#[serde(try_from = "String")]
pub struct RGB(pub u8, pub u8, pub u8);

impl FromStr for RGB {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = s.trim().as_bytes();
        if s.len() != 7 {
            return Err("expected 7 bytes".into());
        }
        if s[0] != b'#' {
            return Err("expected # sign".into());
        }
        let r = unhex(s[1], s[2]).ok_or("expected hex code for red")?;
        let g = unhex(s[3], s[4]).ok_or("expected hex code for green")?;
        let b = unhex(s[5], s[6]).ok_or("expected hex code for blue")?;
        Ok(RGB(r, g, b))
    }
}

impl TryFrom<String> for RGB {
    type Error = String;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl fmt::Display for RGB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('#')?;
        write_hex(f, self.0)?;
        write_hex(f, self.1)?;
        write_hex(f, self.2)?;
        Ok(())
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct IndicatorPack<S = Style> {
    #[serde(default)]
    pub sync: SyncIndicatorPack<S>,
}

impl<S: Clone> IndicatorPack<S> {
    pub fn merge(&mut self, other: Self, flags: MergeFlags)
    where
        SyncIndicatorPack<S>: Merge,
    {
        self.sync.merge(other.sync, flags);
    }

    pub fn merged(mut self, other: Self, flags: MergeFlags) -> Self
    where
        SyncIndicatorPack<S>: Merge,
    {
        self.merge(other, flags);
        self
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct SyncIndicatorPack<S = Style> {
    #[serde(default)]
    pub synced: Indicator<S>,
    #[serde(default)]
    pub failed: Indicator<S>,
}

// SyncIndicatorPack Mergeable impls are in v1 module

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct Indicator<S = Style> {
    #[serde(default)]
    pub outer: IndicatorStyle<S>,
    #[serde(default)]
    pub inner: IndicatorStyle<S>,
    #[serde(default)]
    pub text: String,
}

// Indicator merge methods are in v1 module

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct IndicatorStyle<S = Style> {
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub style: S,
}

// Trait for types that support merging
pub trait Merge<T = Self> {
    fn merge(&mut self, other: T, flags: MergeFlags);
    fn merged(self, other: T, flags: MergeFlags) -> Self
    where
        Self: Sized,
    {
        let mut result = self;
        result.merge(other, flags);
        result
    }
}

// Convenience alias for the common case of merging with references
pub trait MergedWith<T>: Merge<T> {}

// ---

/// Theme version with major.minor components
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ThemeVersion {
    pub major: u32,
    pub minor: u32,
}

impl ThemeVersion {
    /// Create a new theme version
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Version 0.0 (implicit, no version field in theme)
    pub const V0_0: Self = Self { major: 0, minor: 0 };

    /// Version 1.0 (first versioned theme format)
    pub const V1_0: Self = Self { major: 1, minor: 0 };

    /// Current supported version
    pub const CURRENT: Self = Self::V1_0;

    /// Check if this version is compatible with a supported version
    pub fn is_compatible_with(&self, supported: &ThemeVersion) -> bool {
        // Same major version and minor <= supported
        self.major == supported.major && self.minor <= supported.minor
    }
}

impl FromStr for ThemeVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        let err = || Error::InvalidVersion(s.into());

        if parts.len() != 2 {
            return Err(err());
        }

        let major: u32 = parts[0].parse().map_err(|_| err())?;
        let minor: u32 = parts[1].parse().map_err(|_| err())?;

        // Reject leading zeros (except "0" itself)
        if (parts[0].len() > 1 && parts[0].starts_with('0')) || (parts[1].len() > 1 && parts[1].starts_with('0')) {
            return Err(err());
        }

        Ok(ThemeVersion { major, minor })
    }
}

impl fmt::Display for ThemeVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl Serialize for ThemeVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ThemeVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ThemeVersionVisitor;

        impl<'de> Visitor<'de> for ThemeVersionVisitor {
            type Value = ThemeVersion;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a version string like \"1.0\"")
            }

            fn visit_str<E>(self, value: &str) -> Result<ThemeVersion, E>
            where
                E: serde::de::Error,
            {
                ThemeVersion::from_str(value).map_err(|e| E::custom(format!("invalid version: {}", e)))
            }
        }

        deserializer.deserialize_str(ThemeVersionVisitor)
    }
}

// ---

// ---

#[derive(RustEmbed)]
#[folder = "etc/defaults/themes/"]
struct Assets;

// ---

fn unhex(high: u8, low: u8) -> Option<u8> {
    unhex_one(high).and_then(|high| unhex_one(low).map(|low| (high << 4) + low))
}

fn unhex_one(v: u8) -> Option<u8> {
    match v {
        b'0'..=b'9' => Some(v - b'0'),
        b'a'..=b'f' => Some(10 + v - b'a'),
        b'A'..=b'F' => Some(10 + v - b'A'),
        _ => None,
    }
}

fn write_hex<T: fmt::Write>(to: &mut T, v: u8) -> fmt::Result {
    to.write_char(HEXDIGIT[(v >> 4) as usize].into())?;
    to.write_char(HEXDIGIT[(v & 0xF) as usize].into())?;
    Ok(())
}

const HEXDIGIT: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

// ---

#[cfg(test)]
pub mod testing {
    use super::*;

    #[derive(RustEmbed)]
    #[folder = "src/testing/assets/themes/"]
    struct Assets;

    pub fn theme() -> Result<Theme> {
        Theme::load_embedded::<Assets>("test")?
            .resolve()
            .map_err(|source| Error::FailedToResolveTheme {
                name: "test".into(),
                source,
            })
    }
}

#[cfg(test)]
mod tests;
