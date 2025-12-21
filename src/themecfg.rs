// std imports
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::{self, Write},
    hash::Hash,
    io::{self, ErrorKind},
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
    Deserialize, Deserializer,
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

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown theme {name}", name=.name.hlq())]
    ThemeNotFound { name: String, suggestions: Suggestions },
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
    Emphasized,
    Muted,
    Accent,
    AccentSecondary,
    Syntax,
    Status,
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
    pub styles: StylePack<Role>,
    pub elements: StylePack,
    pub levels: HashMap<InfallibleLevel, StylePack>,
    pub indicators: IndicatorPack,
}

impl Theme {
    pub fn load(app_dirs: &AppDirs, name: &str) -> Result<Self> {
        const DEFAULT_THEME_NAME: &str = "@default";

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
        self.styles.merge(other.styles);
        self.elements.merge(other.elements);

        for (level, pack) in other.levels {
            self.levels
                .entry(level)
                .and_modify(|existing| existing.merge(pack.clone()))
                .or_insert(pack);
        }

        self.tags = other.tags;
        self.indicators.merge(other.indicators);
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
            name: name.to_string(),
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
                    return Self::from_buf(&data, format).map_err(|e| map_err(e, path));
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
        Assets::iter().filter_map(|a| Self::strip_known_extension(&a).map(|n| n.into()))
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

    pub fn merge(&mut self, patch: Self) {
        self.0.extend(patch.0);
    }

    pub fn merged(mut self, patch: Self) -> Self {
        self.merge(patch);
        self
    }
}

impl StylePack<Role, Style> {
    pub fn resolve(&self) -> StylePack<Role, ResolvedStyle> {
        let mut resolver = StyleResolver::new(self);
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

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct Style {
    #[serde(rename = "style")]
    pub base: Option<Role>,
    #[serde(flatten)]
    pub body: ResolvedStyle,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            base: None,
            body: ResolvedStyle::new(),
        }
    }

    pub fn base(self, base: Option<Role>) -> Self {
        Self { base, ..self }
    }

    pub fn modes(self, modes: Vec<Mode>) -> Self {
        Self {
            body: self.body.modes(modes),
            ..self
        }
    }

    pub fn background(self, color: Option<Color>) -> Self {
        Self {
            body: self.body.background(color),
            ..self
        }
    }

    pub fn foreground(self, color: Option<Color>) -> Self {
        Self {
            body: self.body.foreground(color),
            ..self
        }
    }

    pub fn merged(mut self, other: &Self) -> Self {
        if let Some(base) = other.base {
            self.base = Some(base);
        }
        self.body = self.body.merged(&other.body);
        self
    }

    pub fn resolve(&self, inventory: &StylePack<Role, ResolvedStyle>) -> ResolvedStyle {
        if let Some(base) = self.base {
            if let Some(base) = inventory.0.get(&base) {
                return base.clone().merged(&self.body);
            }
        }

        self.body.clone()
    }
}

impl Default for &Style {
    fn default() -> Self {
        static DEFAULT: Style = Style::new();
        &DEFAULT
    }
}

// ---

impl From<Role> for Style {
    fn from(base: Role) -> Self {
        Self {
            base: Some(base),
            body: Default::default(),
        }
    }
}

impl From<ResolvedStyle> for Style {
    fn from(body: ResolvedStyle) -> Self {
        Self { base: None, body }
    }
}

// ---

pub struct StyleResolver<'a> {
    inventory: &'a StylePack<Role, Style>,
    cache: HashMap<Role, ResolvedStyle>,
    depth: usize,
}

impl<'a> StyleResolver<'a> {
    fn new(inventory: &'a StylePack<Role, Style>) -> Self {
        Self {
            inventory,
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
            return style.body.clone();
        }

        self.depth += 1;
        let resolved = self.resolve_style(style, role);
        self.depth -= 1;

        self.cache.insert(*role, resolved.clone());

        resolved
    }

    fn resolve_style(&mut self, style: &Style, role: &Role) -> ResolvedStyle {
        let base = style.base.or_else(|| {
            if *role != Role::Default {
                Some(Role::Default)
            } else {
                None
            }
        });

        if let Some(base) = base {
            return self.resolve(&base).merged(&style.body);
        }

        style.body.clone()
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct ResolvedStyle {
    pub modes: Vec<Mode>,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

impl ResolvedStyle {
    pub const fn new() -> Self {
        Self {
            modes: Vec::new(),
            foreground: None,
            background: None,
        }
    }

    pub fn modes(self, modes: Vec<Mode>) -> Self {
        Self { modes, ..self }
    }

    pub fn foreground(self, foreground: Option<Color>) -> Self {
        Self { foreground, ..self }
    }

    pub fn background(self, background: Option<Color>) -> Self {
        Self { background, ..self }
    }

    pub fn merged(mut self, other: &Self) -> Self {
        if !other.modes.is_empty() {
            self.modes = other.modes.clone()
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
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
    pub fn merge(&mut self, other: Self) {
        self.sync.merge(other.sync);
    }

    pub fn merged(mut self, other: Self) -> Self {
        self.merge(other);
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
                        modes: vec![Mode::Bold],
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
    pub fn merge(&mut self, other: Self) {
        self.synced.merge(other.synced);
        self.failed.merge(other.failed);
    }

    pub fn merged(mut self, other: Self) -> Self {
        self.merge(other);
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
    pub fn merge(&mut self, other: Self) {
        self.outer.merge(other.outer);
        self.inner.merge(other.inner);
        if !other.text.is_empty() {
            self.text = other.text;
        }
    }

    pub fn merged(mut self, other: Self) -> Self {
        self.merge(other);
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
    pub fn merge(&mut self, other: Self) {
        if !other.prefix.is_empty() {
            self.prefix = other.prefix;
        }
        if !other.suffix.is_empty() {
            self.suffix = other.suffix;
        }
        self.style = std::mem::take(&mut self.style).merged(&other.style);
    }

    pub fn merged(mut self, other: Self) -> Self {
        self.merge(other);
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
