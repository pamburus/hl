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
use strum::{Display, EnumIter, IntoEnumIterator};
use thiserror::Error;
use yaml_peg::{NodeRc as YamlNode, serde as yaml};

// local imports
use crate::{
    appdirs::AppDirs,
    level::InfallibleLevel,
    xerr::{HighlightQuoted, Suggestions},
};

// Private constants
const DEFAULT_THEME_NAME: &str = "@default";

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

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown theme {name}", name=.name.hlq())]
    ThemeNotFound { name: Arc<str>, suggestions: Suggestions },
    #[error("failed to load theme {name}: {source}", name=.name.hlq())]
    FailedToLoadEmbeddedTheme { name: Arc<str>, source: ExternalError },
    #[error("failed to load theme {name} from {path}: {source}", name=.name.hlq(), path=.path.hlq())]
    FailedToLoadCustomTheme {
        name: Arc<str>,
        path: Arc<Path>,
        source: ExternalError,
    },
    #[error("failed to list custom themes: {0}")]
    FailedToListCustomThemes(#[from] io::Error),
    #[error("invalid tag {value}", value=.value.hlq())]
    InvalidTag { value: Arc<str>, suggestions: Suggestions },
    #[error("style recursion limit exceeded")]
    StyleRecursionLimitExceeded,
    #[error("theme version {requested} is not supported (maximum supported: {supported})")]
    UnsupportedVersion {
        requested: ThemeVersion,
        supported: ThemeVersion,
    },
    #[error("invalid version format: {0}")]
    InvalidVersion(Arc<str>),
}

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum ExternalError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("failed to parse yaml: {0}")]
    YamlSerdeError(#[from] yaml::SerdeError),
    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
    #[error("failed to parse json: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("failed to parse utf-8 string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// ---

#[repr(u8)]
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Enum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Default,
    Primary,
    Secondary,
    Strong,
    Muted,
    Accent,
    AccentSecondary,
    Message,
    Syntax,
    Status,
    Level,
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

// ---

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Theme {
    #[serde(deserialize_with = "enumset_serde::deserialize")]
    pub tags: EnumSet<Tag>,
    pub version: ThemeVersion,
    pub styles: StylePack<Role>,
    pub elements: StylePack,
    pub levels: HashMap<InfallibleLevel, StylePack>,
    pub indicators: IndicatorPack,
}

impl Theme {
    pub fn load(app_dirs: &AppDirs, name: &str) -> Result<Self> {
        let theme = Self::load_embedded::<Assets>(DEFAULT_THEME_NAME)?;
        if name == DEFAULT_THEME_NAME {
            return Ok(theme);
        }

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

    pub fn merge(&mut self, other: Self) {
        let flags = other.merge_flags();
        self.version = other.version;
        self.styles.merge(other.styles);

        // Apply blocking rules only for version 0 themes (backward compatibility)
        if flags.contains(MergeFlag::ReplaceGroups) {
            // Apply blocking rule: if child theme defines a parent element,
            // remove the corresponding -inner element from parent theme
            let parent_inner_pairs = [
                (Element::Level, Element::LevelInner),
                (Element::Logger, Element::LoggerInner),
                (Element::Caller, Element::CallerInner),
                (Element::InputNumber, Element::InputNumberInner),
                (Element::InputName, Element::InputNameInner),
            ];

            // Block base -inner elements if parent is defined in child theme
            for (parent, inner) in parent_inner_pairs {
                if other.elements.0.contains_key(&parent) {
                    self.elements.0.remove(&inner);
                }
            }

            // Block input-number/input-name and their inner elements if input is defined in child theme
            // This ensures v0 themes that define `input` get nested styling scope behavior
            if other.elements.0.contains_key(&Element::Input) {
                self.elements.0.remove(&Element::InputNumber);
                self.elements.0.remove(&Element::InputNumberInner);
                self.elements.0.remove(&Element::InputName);
                self.elements.0.remove(&Element::InputNameInner);
            }

            // Block entire level sections if child theme defines any element for that level
            for level in other.levels.keys() {
                self.levels.remove(level);
            }
        }

        // For both v0 and v1, elements defined in child theme replace elements from parent theme
        // Property-level merge happens later when merging elements with per-level styles
        self.elements.0.extend(other.elements.0);

        // For both v0 and v1, level-specific elements defined in child theme replace from parent
        for (level, pack) in other.levels {
            self.levels
                .entry(level)
                .and_modify(|existing| existing.0.extend(pack.0.clone()))
                .or_insert(pack);
        }

        self.tags = other.tags;
        self.indicators.merge(other.indicators, flags);
    }

    pub fn merged(mut self, other: Self) -> Self {
        self.merge(other);
        self
    }

    pub fn embedded(name: &str) -> Result<Self> {
        Self::load_embedded::<Assets>(name)
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

    pub fn merge_flags(&self) -> MergeFlags {
        match self.version {
            ThemeVersion { major: 0, .. } => {
                MergeFlag::ReplaceElements | MergeFlag::ReplaceGroups | MergeFlag::ReplaceModes
            }
            _ => MergeFlags::new(),
        }
    }

    fn validate_version(&self) -> Result<()> {
        if self.version == ThemeVersion::default() {
            // Version 0.0 (no version field) is considered compatible
            return Ok(());
        }
        if !self.version.is_compatible_with(&ThemeVersion::CURRENT) {
            return Err(Error::UnsupportedVersion {
                requested: self.version,
                supported: ThemeVersion::CURRENT,
            });
        }

        Ok(())
    }

    fn load_embedded<S: RustEmbed>(name: &str) -> Result<Self> {
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

    fn from_buf(data: &[u8], format: Format) -> Result<Self, ExternalError> {
        let s = std::str::from_utf8(data)?;
        match format {
            Format::Yaml => Ok(yaml::from_str(s)?.remove(0)),
            Format::Toml => Ok(toml::from_str(s)?),
            Format::Json => Ok(json::from_str(s)?),
        }
    }

    fn load_from(dir: &Path, name: &str) -> Result<Self> {
        for format in Format::iter() {
            let filename = Self::filename(name, format);
            let path = PathBuf::from(&filename);
            let path = if matches!(path.components().next(), Some(Component::ParentDir | Component::CurDir)) {
                path
            } else {
                dir.join(&filename)
            };

            let map_err = |e: ExternalError, path: PathBuf| Error::FailedToLoadCustomTheme {
                name: name.into(),
                path: path.into(),
                source: e,
            };

            match std::fs::read(&path) {
                Ok(data) => {
                    let theme = Self::from_buf(&data, format).map_err(|e| map_err(e, path))?;
                    theme.validate_version()?;
                    return Ok(theme);
                }
                Err(e) => match e.kind() {
                    ErrorKind::NotFound => continue,
                    _ => return Err(map_err(e.into(), path)),
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

pub type StyleInventory = StylePack<Role, ResolvedStyle>;

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
        S: Clone + for<'a> MergedWith<&'a S>,
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
                .and_modify(|v| *v = v.clone().merged_with(&patch, flags))
                .or_insert(patch);
        }
    }

    pub fn merged(mut self, patch: Self, flags: MergeFlags) -> Self
    where
        S: Clone + for<'a> MergedWith<&'a S>,
    {
        self.merge(patch, flags);
        self
    }
}

impl MergedWith<&StylePack> for StylePack {
    fn merged_with(mut self, other: &StylePack<Element, Style>, flags: MergeFlags) -> Self {
        self.merge(other.clone(), flags);
        self
    }
}

// ---

pub trait MergedWith<T> {
    fn merged_with(self, other: T, flags: MergeFlags) -> Self;
}

#[derive(Debug, Hash, Ord, PartialOrd, EnumSetType, Deserialize)]
pub enum MergeFlag {
    ReplaceElements,
    ReplaceGroups,
    ReplaceModes,
}

pub type MergeFlags = EnumSet<MergeFlag>;

// ---

impl StylePack<Role, Style> {
    pub fn resolve(&self, flags: MergeFlags) -> StylePack<Role, ResolvedStyle> {
        let mut resolver = StyleResolver::new(self, flags);
        let items = self.0.keys().map(|k| (*k, resolver.resolve(k))).collect();
        StylePack(items)
    }
}

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

        while let Some(key) = access.next_key::<YamlNode>()? {
            if let Ok(key) = K::deserialize(key) {
                let value: S = access.next_value()?;
                items.insert(key, value);
            } else {
                _ = access.next_value::<YamlNode>()?;
            }
        }

        Ok(StylePack(items))
    }
}

// ---

#[repr(u8)]
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Enum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Element {
    Input,
    InputNumber,
    InputNumberInner,
    InputName,
    InputNameInner,
    Time,
    Level,
    LevelInner,
    Logger,
    LoggerInner,
    Caller,
    CallerInner,
    Message,
    MessageDelimiter,
    Field,
    Key,
    Array,
    Object,
    String,
    Number,
    Boolean,
    BooleanTrue,
    BooleanFalse,
    Null,
    Ellipsis,
}

impl Element {
    pub fn is_inner(&self) -> bool {
        self.parent().is_some()
    }

    pub fn parent(&self) -> Option<Element> {
        match self {
            Element::InputNumberInner => Some(Element::InputNumber),
            Element::InputNameInner => Some(Element::InputName),
            Element::LevelInner => Some(Element::Level),
            Element::LoggerInner => Some(Element::Logger),
            Element::CallerInner => Some(Element::Caller),
            _ => None,
        }
    }

    pub fn pairs() -> &'static [(Element, Element)] {
        &[
            (Element::InputNumber, Element::InputNumberInner),
            (Element::InputName, Element::InputNameInner),
            (Element::Level, Element::LevelInner),
            (Element::Logger, Element::LoggerInner),
            (Element::Caller, Element::CallerInner),
        ]
    }
}

// ---

/// Represents one or more base styles for inheritance.
/// Supports both single role (`style = "warning"`) and multiple roles (`style = ["primary", "warning"]`).
/// When multiple roles are specified, they are merged left to right (later roles override earlier ones).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StyleBase(pub Vec<Role>);

impl StyleBase {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Role> {
        self.0.iter()
    }
}

impl From<Role> for StyleBase {
    fn from(role: Role) -> Self {
        Self(vec![role])
    }
}

impl From<Vec<Role>> for StyleBase {
    fn from(roles: Vec<Role>) -> Self {
        Self(roles)
    }
}

impl<'de> Deserialize<'de> for StyleBase {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, SeqAccess, Visitor};

        struct StyleBaseVisitor;

        impl<'de> Visitor<'de> for StyleBaseVisitor {
            type Value = StyleBase;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a role name or array of role names")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                let role: Role = serde_plain::from_str(value).map_err(de::Error::custom)?;
                Ok(StyleBase(vec![role]))
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut roles = Vec::new();
                while let Some(value) = seq.next_element::<String>()? {
                    let role: Role = serde_plain::from_str(&value).map_err(de::Error::custom)?;
                    roles.push(role);
                }
                Ok(StyleBase(roles))
            }
        }

        deserializer.deserialize_any(StyleBaseVisitor)
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct Style {
    #[serde(rename = "style")]
    pub base: StyleBase,
    pub modes: ModeSetDiff,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            base: StyleBase(Vec::new()),
            modes: ModeSetDiff::new(),
            foreground: None,
            background: None,
        }
    }

    pub fn base(self, base: impl Into<StyleBase>) -> Self {
        Self {
            base: base.into(),
            ..self
        }
    }

    pub fn modes(self, modes: ModeSetDiff) -> Self {
        Self { modes, ..self }
    }

    pub fn background(self, background: Option<Color>) -> Self {
        Self { background, ..self }
    }

    pub fn foreground(self, foreground: Option<Color>) -> Self {
        Self { foreground, ..self }
    }

    pub fn merged(mut self, other: &Self, flags: MergeFlags) -> Self {
        if !other.base.is_empty() {
            self.base = other.base.clone();
        }
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
        self
    }

    pub fn resolve(&self, inventory: &StylePack<Role, ResolvedStyle>, flags: MergeFlags) -> ResolvedStyle {
        Self::resolve_with(&self.base, self, flags, |role| {
            inventory.0.get(role).cloned().unwrap_or_default()
        })
    }

    fn resolve_with<F>(bases: &StyleBase, style: &Style, flags: MergeFlags, mut resolve_role: F) -> ResolvedStyle
    where
        F: FnMut(&Role) -> ResolvedStyle,
    {
        if bases.is_empty() {
            return style.as_resolved();
        }

        // Resolve multiple bases: merge left to right, then apply style on top
        let mut result = ResolvedStyle::default();
        for role in bases.iter() {
            result = result.merged_with(&resolve_role(role), flags);
        }
        result.merged_with(style, flags)
    }

    fn as_resolved(&self) -> ResolvedStyle {
        ResolvedStyle {
            modes: self.modes,
            foreground: self.foreground,
            background: self.background,
        }
    }
}

impl Default for &Style {
    fn default() -> Self {
        static DEFAULT: Style = Style::new();
        &DEFAULT
    }
}

impl MergedWith<&Style> for Style {
    fn merged_with(self, other: &Style, flags: MergeFlags) -> Self {
        self.merged(other, flags)
    }
}

impl From<Role> for Style {
    fn from(role: Role) -> Self {
        Self {
            base: StyleBase::from(role),
            ..Default::default()
        }
    }
}

impl From<Vec<Role>> for Style {
    fn from(roles: Vec<Role>) -> Self {
        Self {
            base: StyleBase::from(roles),
            ..Default::default()
        }
    }
}

impl From<ResolvedStyle> for Style {
    fn from(body: ResolvedStyle) -> Self {
        Self {
            base: StyleBase::default(),
            modes: body.modes,
            foreground: body.foreground,
            background: body.background,
        }
    }
}

// ---

pub struct StyleResolver<'a> {
    inventory: &'a StylePack<Role, Style>,
    flags: MergeFlags,
    cache: HashMap<Role, ResolvedStyle>,
    depth: usize,
}

impl<'a> StyleResolver<'a> {
    fn new(inventory: &'a StylePack<Role, Style>, flags: MergeFlags) -> Self {
        Self {
            inventory,
            flags,
            cache: HashMap::new(),
            depth: 0,
        }
    }

    fn resolve(&mut self, role: &Role) -> ResolvedStyle {
        if let Some(resolved) = self.cache.get(role) {
            return resolved.clone();
        }

        let style = self.inventory.0.get(role).unwrap_or_default();

        if self.depth >= RECURSION_LIMIT {
            log::warn!("style recursion limit exceeded for style {:?}", &role);
            return style.as_resolved();
        }

        self.depth += 1;
        let resolved = self.resolve_style(style, role);
        self.depth -= 1;

        self.cache.insert(*role, resolved.clone());

        resolved
    }

    fn resolve_style(&mut self, style: &Style, role: &Role) -> ResolvedStyle {
        // If no explicit base, default to inheriting from Default role (except for Default itself)
        let bases = if style.base.is_empty() {
            if *role != Role::Default {
                StyleBase::from(Role::Default)
            } else {
                StyleBase::default()
            }
        } else {
            style.base.clone()
        };

        Style::resolve_with(&bases, style, self.flags, |r| self.resolve(r))
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct ResolvedStyle {
    pub modes: ModeSetDiff,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

impl ResolvedStyle {
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

impl MergedWith<&ResolvedStyle> for ResolvedStyle {
    fn merged_with(mut self, other: &ResolvedStyle, flags: MergeFlags) -> Self {
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
        self
    }
}

impl MergedWith<&Style> for ResolvedStyle {
    fn merged_with(mut self, other: &Style, flags: MergeFlags) -> Self {
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
        self
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
#[serde(default)]
pub struct IndicatorPack {
    pub sync: SyncIndicatorPack,
}

impl IndicatorPack {
    pub fn merge(&mut self, other: Self, flags: MergeFlags) {
        self.sync.merge(other.sync, flags);
    }

    pub fn merged(mut self, other: Self, flags: MergeFlags) -> Self {
        self.merge(other, flags);
        self
    }
}

// ---

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SyncIndicatorPack {
    pub synced: Indicator,
    pub failed: Indicator,
}

impl Default for SyncIndicatorPack {
    fn default() -> Self {
        Self {
            synced: Indicator {
                outer: IndicatorStyle::default(),
                inner: IndicatorStyle::default(),
                text: " ".into(),
            },
            failed: Indicator {
                outer: IndicatorStyle::default(),
                inner: IndicatorStyle {
                    prefix: String::default(),
                    suffix: String::default(),
                    style: ResolvedStyle {
                        modes: Mode::Bold.into(),
                        background: None,
                        foreground: Some(Color::Plain(PlainColor::Yellow)),
                    }
                    .into(),
                },
                text: "!".into(),
            },
        }
    }
}

impl SyncIndicatorPack {
    pub fn merge(&mut self, other: Self, flags: MergeFlags) {
        self.synced.merge(other.synced, flags);
        self.failed.merge(other.failed, flags);
    }

    pub fn merged(mut self, other: Self, flags: MergeFlags) -> Self {
        self.merge(other, flags);
        self
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct Indicator {
    pub outer: IndicatorStyle,
    pub inner: IndicatorStyle,
    pub text: String,
}

impl Indicator {
    pub fn merge(&mut self, other: Self, flags: MergeFlags) {
        self.outer.merge(other.outer, flags);
        self.inner.merge(other.inner, flags);
        if !other.text.is_empty() {
            self.text = other.text;
        }
    }

    pub fn merged(mut self, other: Self, flags: MergeFlags) -> Self {
        self.merge(other, flags);
        self
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct IndicatorStyle {
    pub prefix: String,
    pub suffix: String,
    pub style: Style,
}

impl IndicatorStyle {
    pub fn merge(&mut self, other: Self, flags: MergeFlags) {
        if !other.prefix.is_empty() {
            self.prefix = other.prefix;
        }
        if !other.suffix.is_empty() {
            self.suffix = other.suffix;
        }
        self.style = std::mem::take(&mut self.style).merged(&other.style, flags);
    }

    pub fn merged(mut self, other: Self, flags: MergeFlags) -> Self {
        self.merge(other, flags);
        self
    }
}

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

const RECURSION_LIMIT: usize = 64;

// ---

#[cfg(test)]
pub mod testing {
    use super::*;

    #[derive(RustEmbed)]
    #[folder = "src/testing/assets/themes/"]
    struct Assets;

    pub fn theme() -> Result<Theme> {
        Theme::load_embedded::<Assets>("test")
    }
}

#[cfg(test)]
mod tests;
