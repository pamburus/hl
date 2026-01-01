// std imports
use std::{
    collections::HashMap,
    hash::Hash,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    str::{self, FromStr},
    sync::Arc,
};

// third-party imports
use enum_map::Enum;
use enumset::{EnumSet, EnumSetType};
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json as json;
use strum::{Display, EnumIter, IntoEnumIterator};
use yaml_peg::serde as yaml;

// local imports
use crate::{appdirs::AppDirs, level::Level, xerr::Suggestions};

// relative imports
use super::{Error, ExternalError, IndicatorPack, Merge, RawTheme, Result, StylePack, ThemeLoadError, Version, v0, v1};

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
    pub version: Version,
    pub elements: StylePack,
    pub levels: HashMap<Level, StylePack>,
    pub indicators: IndicatorPack,
}

impl Theme {
    /// Load a fully resolved theme by name.
    ///
    /// This is the primary method for loading themes. It performs the complete
    /// theme loading pipeline:
    /// 1. Loads the theme from custom directory or embedded themes
    /// 2. Merges with the `@base` theme
    /// 3. Resolves all role-based styles to concrete styles
    ///
    /// The theme is searched in the following order:
    /// - Custom themes in `{config_dir}/themes/`
    /// - Embedded themes (built into the binary)
    ///
    /// All themes are automatically merged with `@base` to ensure all
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
    /// - Automatically merges with `@base` theme
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
    pub fn load(dirs: &AppDirs, name: &str) -> Result<Self> {
        Self::load_raw(dirs, name)?
            .merged(Self::load_raw(dirs, "@accent-italic")?)
            .finalized()
            .resolve()
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
    pub fn load_raw(dirs: &AppDirs, name: &str) -> Result<RawTheme> {
        let theme = match Self::load_from(&Self::themes_dir(dirs), name) {
            Ok(v) => Ok(v),
            Err(Error::ThemeNotFound { .. }) => match Self::load_embedded::<Assets>(name) {
                Ok(v) => Ok(v),
                Err(Error::ThemeNotFound { name, mut suggestions }) => {
                    if let Ok(variants) = Self::custom_names(dirs) {
                        let variants = variants.into_iter().filter_map(|v| v.ok());
                        suggestions = suggestions.merge(Suggestions::new(&name, variants));
                    }
                    Err(Error::ThemeNotFound { name, suggestions })
                }
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }?;

        Ok(theme)
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

    pub(super) fn load_embedded<S: RustEmbed>(name: &str) -> Result<RawTheme> {
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

    pub(super) fn from_buf(data: &[u8], format: Format) -> Result<v1::Theme, ThemeLoadError> {
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

    pub(super) fn themes_dir(app_dirs: &AppDirs) -> PathBuf {
        app_dirs.config_dir.join("themes")
    }

    pub(super) fn load_from(dir: &Path, name: &str) -> Result<RawTheme> {
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

    pub(super) fn embedded_names() -> impl IntoIterator<Item = Arc<str>> {
        Assets::iter().filter_map(|a| {
            Self::strip_known_extension(&a)
                .filter(|&n| n.starts_with('@'))
                .map(|n| n.into())
        })
    }

    pub(super) fn custom_names(app_dirs: &AppDirs) -> Result<impl IntoIterator<Item = Result<Arc<str>>> + use<>> {
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

    fn peek_version(s: &str, format: Format) -> Result<Version, ExternalError> {
        #[derive(Deserialize)]
        struct VersionOnly {
            #[serde(default)]
            version: Version,
        }

        let data: VersionOnly = Self::deserialize(s, format)?;
        Ok(data.version)
    }

    fn filename(name: &str, format: Format) -> String {
        if Self::strip_extension(name, format).is_some() {
            return name.to_string();
        }

        format!("{}.{}", name, format.extensions()[0])
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
    Overlay,
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

#[derive(RustEmbed)]
#[folder = "etc/defaults/themes/"]
pub(super) struct Assets;

// ---

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
pub(crate) mod tests;
