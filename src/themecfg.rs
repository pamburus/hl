//! Theme configuration system for `hl`.
//!
//! Handles theme loading, parsing, and resolution for both v0 (legacy) and v1 (semantic) formats.
//!
//! # Main Types
//!
//! - [`Theme`]: Fully resolved theme ready for use.
//! - [`RawTheme`]: Unresolved theme (allows modification before resolution).
//! - [`Role`]: Semantic style role (primary, warning, etc.) - v1 only.
//! - [`Style`]: Concrete style with color and modes.
//!
//! # Usage
//!
//! Load a resolved theme:
//! ```no_run
//! # use hl::themecfg::Theme;
//! # let app_dirs = hl::appdirs::AppDirs::new("hl").unwrap();
//! let theme = Theme::load(&app_dirs, "monokai")?;
//! # Ok::<(), hl::themecfg::Error>(())
//! ```
//!
//! Load raw theme for customization:
//! ```no_run
//! # use hl::themecfg::Theme;
//! # let app_dirs = hl::appdirs::AppDirs::new("hl").unwrap();
//! let raw = Theme::load_raw(&app_dirs, "monokai")?;
//! let theme = raw.resolve()?;
//! # Ok::<(), hl::themecfg::Error>(())
//! ```

// std imports
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt,
    hash::Hash,
    io::ErrorKind,
    ops::{Add, AddAssign},
    path::{Component, Path, PathBuf},
    str::{self, FromStr},
    sync::Arc,
};

// third-party imports
use enum_map::Enum;
use enumset::{EnumSet, EnumSetType};
use rust_embed::RustEmbed;
use serde::{Deserialize, Deserializer, Serialize, de::Visitor};
use serde_json as json;
use strum::{Display, EnumIter, IntoEnumIterator};
use thiserror::Error;
use yaml_peg::serde as yaml;

// local imports
use crate::{appdirs::AppDirs, level::Level, xerr::Suggestions};

// Version-specific modules
pub mod error;
pub mod raw;
pub mod v0;
pub mod v1;

pub use error::{Error, ExternalError, Result, ThemeLoadError};
pub use raw::RawTheme;

// Re-export v1 types that are part of the public API
// (Element comes from v0, re-exported by v1)
pub use v1::{Element, Role, StyleBase};

// Re-export v1::StylePack for use with resolved styles
// This is the generic container used throughout the system
pub use v1::StylePack;

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
    /// # Usage
    ///
    /// Call with application directories and theme name:
    /// - `Theme::load(app_dirs, "monokai")` loads the monokai theme
    /// - Searches custom directory first, then embedded themes
    /// - Automatically merges with `@default` theme
    /// - Returns fully resolved theme ready for use
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
        Self::load_raw(app_dirs, name)?.resolve()
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
    /// # Usage
    ///
    /// For advanced theme customization:
    /// 1. `let mut raw = Theme::load_raw(app_dirs, "monokai")?;`
    /// 2. Modify the theme: `raw.styles.0.insert(role, style);`
    /// 3. Resolve: `let theme = raw.resolve()?;`
    ///
    /// The returned `RawTheme` includes theme metadata, so any resolution
    /// errors will automatically include the theme name and source.
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
        let default_theme = Self::load_embedded::<Assets>(DEFAULT_THEME_NAME)?;

        let theme = match Self::load_from(&Self::themes_dir(app_dirs), name) {
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
        }?;

        Ok(default_theme.merged(theme))
    }

    pub fn embedded(name: &str) -> Result<Self> {
        Self::load_embedded::<Assets>(name)?.resolve()
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
                let inner =
                    Self::from_buf(file.data.as_ref(), format).map_err(|e| Error::FailedToLoadEmbeddedTheme {
                        name: name.into(),
                        source: e,
                    })?;
                let info = ThemeInfo::new(name, ThemeSource::Embedded, ThemeOrigin::Stock);
                return Ok(RawTheme::new(info, inner));
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

    fn from_buf(data: &[u8], format: Format) -> Result<v1::Theme, ThemeLoadError> {
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
            v1::Theme::validate_version(&version)?;
            // V1+ themes use strict deserialization
            let theme: v1::Theme = Self::deserialize(s, format)?;
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
        if name.contains('/') && Self::strip_known_extension(name).is_none() {
            return Err(Error::UnsupportedFileType {
                path: name.into(),
                extension: Arc::from(
                    Path::new(name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("unknown"),
                ),
            });
        }

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
                    let inner = Self::from_buf(&data, format).map_err(|e| map_err(e, &path))?;
                    let info = ThemeInfo::new(
                        name,
                        ThemeSource::Custom {
                            path: path.clone().into(),
                        },
                        ThemeOrigin::Custom,
                    );
                    return Ok(RawTheme::new(info, inner));
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

        format!("{}.{}", name, format.extensions()[0])
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
        for ext in format.extensions() {
            if let Some(name) = filename.strip_suffix(ext).and_then(|r| r.strip_suffix(".")) {
                return Some(name);
            }
        }
        None
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
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::Yaml => &["yaml", "yml"],
            Self::Toml => &["toml"],
            Self::Json => &["json"],
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
/// Theme source information (embedded or custom file).
///
/// This enum tracks where a theme was loaded from, enabling better
/// error messages when theme resolution or validation fails.
#[derive(Debug, Clone)]
pub enum ThemeSource {
    /// Theme embedded in the binary.
    Embedded,
    /// Theme loaded from a custom file.
    Custom { path: Arc<Path> },
}

// ---

/// Theme metadata for error reporting and information display.
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    /// The theme name.
    pub name: Arc<str>,
    /// The theme source (embedded or custom file).
    pub source: ThemeSource,
    /// The theme origin (stock or custom).
    pub origin: ThemeOrigin,
}

impl ThemeInfo {
    /// Create a new ThemeInfo.
    pub fn new(name: impl Into<Arc<str>>, source: ThemeSource, origin: ThemeOrigin) -> Self {
        Self {
            name: name.into(),
            source,
            origin,
        }
    }
}

impl From<ThemeOrigin> for ThemeInfo {
    fn from(origin: ThemeOrigin) -> Self {
        Self {
            name: "unknown".into(),
            source: ThemeSource::Embedded,
            origin,
        }
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

/// Type alias for a style inventory mapping roles to resolved styles.
/// This is the output type after resolution in v1.
pub type StyleInventory = StylePack<Role, Style>;

// ---

#[derive(Debug, Hash, Ord, PartialOrd, EnumSetType, Deserialize)]
pub enum MergeFlag {
    ReplaceElements,
    ReplaceHierarchies,
    ReplaceModes,
}

pub type MergeFlags = EnumSet<MergeFlag>;

// ---

/// A fully resolved style with concrete values.
///
/// This is the output type after resolving [`RawStyle`] (which may contain
/// role references and mode diffs). All values are concrete:
/// - `modes` contains the final mode operations to apply
/// - `foreground` and `background` are final computed colors
#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
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
    let h = (high as char).to_digit(16)?;
    let l = (low as char).to_digit(16)?;
    Some((h as u8) << 4 | (l as u8))
}

#[cfg(test)]
pub mod testing {
    use super::*;

    #[derive(RustEmbed)]
    #[folder = "src/testing/assets/themes/"]
    struct Assets;

    pub fn theme() -> Result<Theme> {
        Theme::load_embedded::<Assets>("test")?.resolve()
    }
}

#[cfg(test)]
mod tests;
