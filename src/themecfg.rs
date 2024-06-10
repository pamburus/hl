// std imports
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::{self, Write},
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    str::{self, FromStr},
};

// third-party imports
use derive_deref::Deref;
use enum_map::Enum;
use platform_dirs::AppDirs;
use rust_embed::RustEmbed;
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json as json;
use serde_yaml as yaml;
use strum::{EnumIter, IntoEnumIterator};

// local imports
use crate::{error::*, level::Level};

// ---

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub elements: StylePack,
    pub levels: HashMap<Level, StylePack>,
    pub indicators: IndicatorPack,
}

impl Theme {
    pub fn load(app_dirs: &AppDirs, name: &str) -> Result<Self> {
        match Self::load_from(&Self::themes_dir(app_dirs), name) {
            Err(Error::Io(e)) => match e.kind() {
                ErrorKind::NotFound => match Self::load_embedded::<Assets>(name) {
                    Err(Error::UnknownTheme { name, mut known }) => {
                        if let Some(names) = Self::custom_names(app_dirs).ok() {
                            known.extend(names.into_iter().filter_map(|n| n.ok()));
                        }
                        known.sort_unstable();
                        known.dedup();
                        Err(Error::UnknownTheme { name, known })
                    }
                    Err(e) => Err(e),
                    Ok(v) => Ok(v),
                },
                _ => Err(Error::Io(e)),
            },
            Err(e) => Err(e),
            Ok(v) => Ok(v),
        }
    }

    pub fn embedded(name: &str) -> Result<Self> {
        Self::load_embedded::<Assets>(name)
    }

    pub fn list(app_dirs: &AppDirs) -> Result<HashMap<String, ThemeInfo>> {
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
                return Self::from_buf(file.data.as_ref(), format);
            }
        }

        Err(Error::UnknownTheme {
            name: name.to_string(),
            known: Self::embedded_names().into_iter().collect(),
        })
    }

    fn from_buf(data: &[u8], format: Format) -> Result<Self> {
        let s = std::str::from_utf8(data)?;
        match format {
            Format::Yaml => Ok(yaml::from_str(s)?),
            Format::Toml => Ok(toml::from_str(s)?),
            Format::Json => Ok(json::from_str(s)?),
        }
    }

    fn load_from(dir: &PathBuf, name: &str) -> Result<Self> {
        for format in Format::iter() {
            let filename = Self::filename(name, format);
            let path = PathBuf::from(&filename);
            let path = if matches!(path.components().next(), Some(Component::ParentDir | Component::CurDir)) {
                path
            } else {
                dir.join(&filename)
            };
            match std::fs::read(&path) {
                Ok(data) => {
                    return Self::from_buf(&data, format).map_err(|e| Error::FailedToLoadTheme {
                        name: name.to_string(),
                        filename: path.display().to_string(),
                        source: Box::new(e),
                    });
                }
                Err(e) => match e.kind() {
                    ErrorKind::NotFound => continue,
                    _ => return Err(e.into()),
                },
            }
        }

        Err(std::io::Error::new(ErrorKind::NotFound, "theme file not found").into())
    }

    fn filename(name: &str, format: Format) -> String {
        if Self::strip_extension(&name, format).is_some() {
            return name.to_string();
        }

        format!("{}.{}", name, format.extension())
    }

    fn themes_dir(app_dirs: &AppDirs) -> PathBuf {
        app_dirs.config_dir.join("themes")
    }

    fn embedded_names() -> impl IntoIterator<Item = String> {
        Assets::iter().filter_map(|a| Self::strip_known_extension(&a).map(|n| n.to_string()))
    }

    fn custom_names(app_dirs: &AppDirs) -> Result<impl IntoIterator<Item = Result<String>>> {
        let path = Self::themes_dir(app_dirs);
        let dir = Path::new(&path);
        Ok(dir
            .read_dir()?
            .map(|item| {
                Ok(item?
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|a| Self::strip_known_extension(&a).map(|n| n.to_string())))
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ThemeOrigin {
    Stock,
    Custom,
}

// ---

#[derive(Clone, Debug, Default, Deref)]
pub struct StylePack(HashMap<Element, Style>);

impl StylePack {
    pub fn items(&self) -> &HashMap<Element, Style> {
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

impl<I: Into<HashMap<Element, Style>>> From<I> for StylePack {
    fn from(i: I) -> Self {
        Self(i.into())
    }
}

impl<'de> Deserialize<'de> for StylePack {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_map(StylePackDeserializeVisitor::new())?)
    }
}

// ---

struct StylePackDeserializeVisitor {}

impl StylePackDeserializeVisitor {
    #[inline]
    fn new() -> Self {
        Self {}
    }
}

impl<'de> Visitor<'de> for StylePackDeserializeVisitor {
    type Value = StylePack;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("style pack object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> std::result::Result<Self::Value, A::Error> {
        let mut items = HashMap::new();

        while let Some(key) = access.next_key::<yaml::Value>()? {
            if let Ok(key) = yaml::from_value(key) {
                let value: Style = access.next_value()?;
                items.insert(key, value);
            } else {
                _ = access.next_value::<yaml::Value>()?;
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
    Field,
    Key,
    Array,
    Object,
    String,
    Number,
    Boolean,
    Null,
    Ellipsis,
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct Style {
    pub modes: Vec<Mode>,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
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
                    style: Style {
                        modes: vec![Mode::Bold],
                        background: None,
                        foreground: Some(Color::Plain(PlainColor::Yellow)),
                    },
                },
                text: "!".into(),
            },
        }
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

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct IndicatorStyle {
    pub prefix: String,
    pub suffix: String,
    pub style: Style,
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
mod tests {
    use super::*;

    #[test]
    fn test_load() {
        let app_dirs = AppDirs {
            config_dir: PathBuf::from("src/testing/assets"),
            cache_dir: Default::default(),
            data_dir: Default::default(),
            state_dir: Default::default(),
        };
        assert_ne!(Theme::load(&app_dirs, "test").unwrap().elements.len(), 0);
        assert_ne!(Theme::load(&app_dirs, "universal").unwrap().elements.len(), 0);
        assert!(Theme::load(&app_dirs, "non-existent").is_err());
        assert!(Theme::load(&app_dirs, "invalid").is_err());
        assert!(Theme::load(&app_dirs, "invalid-type").is_err());
    }

    #[test]
    fn test_load_from() {
        let path = PathBuf::from("etc/defaults/themes");
        assert_ne!(Theme::load_from(&path, "universal").unwrap().elements.len(), 0);

        let path = PathBuf::from("src/testing/assets/themes");
        assert_ne!(Theme::load_from(&path, "test").unwrap().elements.len(), 0);
        assert_ne!(Theme::load_from(&path, "test.toml").unwrap().elements.len(), 0);
        assert_ne!(
            Theme::load_from(&path, "./src/testing/assets/themes/test.toml")
                .unwrap()
                .elements
                .len(),
            0
        );
        assert!(Theme::load_from(&path, "non-existent").is_err());
        assert!(Theme::load_from(&path, "invalid").is_err());
        assert!(Theme::load_from(&path, "invalid-type").is_err());
    }

    #[test]
    fn test_embedded() {
        assert_ne!(Theme::embedded("universal").unwrap().elements.len(), 0);
        assert!(Theme::embedded("non-existent").is_err());
    }

    #[test]
    fn test_rgb() {
        let a = RGB::from_str("#102030").unwrap();
        assert_eq!(a, RGB(16, 32, 48));
        let b: RGB = serde_json::from_str(r##""#102030""##).unwrap();
        assert_eq!(b, RGB(16, 32, 48));
    }

    #[test]
    fn test_style_pack() {
        assert_eq!(StylePack::default().clone().len(), 0);

        let yaml = r##"
            input:
              foreground: red
              background: blue
              modes:
                - bold
                - faint
            message:
              foreground: green
              modes:
                - italic
                - underline
            unexpected: {}
        "##;
        let pack: StylePack = yaml::from_str(yaml).unwrap();
        assert_eq!(pack.0.len(), 2);
        assert_eq!(pack.0[&Element::Input].foreground, Some(Color::Plain(PlainColor::Red)));
        assert_eq!(pack.0[&Element::Input].background, Some(Color::Plain(PlainColor::Blue)));
        assert_eq!(pack.0[&Element::Input].modes, vec![Mode::Bold, Mode::Faint]);
        assert_eq!(
            pack.0[&Element::Message].foreground,
            Some(Color::Plain(PlainColor::Green))
        );
        assert_eq!(pack.0[&Element::Message].background, None);
        assert_eq!(pack.0[&Element::Message].modes, vec![Mode::Italic, Mode::Underline]);

        assert!(yaml::from_str::<StylePack>("invalid")
            .unwrap_err()
            .to_string()
            .ends_with("expected style pack object"));
    }
}
