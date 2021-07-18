// std imports
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::{self, Write},
    io::ErrorKind,
    path::PathBuf,
    str::{self, FromStr},
};

// third-party imports
use derive_deref::Deref;
use enum_map::Enum;
use platform_dirs::AppDirs;
use rust_embed::RustEmbed;
use serde::Deserialize;

// local imports
use crate::{error::*, types::Level};

// ---

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub default: StylePack,
    pub levels: HashMap<Level, StylePack>,
}

impl Theme {
    pub fn load(app_dirs: &AppDirs, name: &str) -> Result<Self> {
        let filename = Self::filename(name);
        match Self::load_from(app_dirs.config_dir.join("themes"), &filename) {
            Err(Error::Io(error)) => match error.kind() {
                ErrorKind::NotFound => Self::from_buf(
                    Asset::get(&filename)
                        .ok_or_else(|| Error::UnknownTheme(name.to_string()))?
                        .as_ref(),
                ),
                _ => Err(Error::Io(error)),
            },
            Err(error) => Err(error),
            Ok(result) => Ok(result),
        }
    }

    fn from_buf(data: &[u8]) -> Result<Self> {
        Ok(serde_yaml::from_str(std::str::from_utf8(data)?)?)
    }
    fn load_from(dir: PathBuf, filename: &str) -> Result<Self> {
        let f = std::fs::File::open(dir.join(filename))?;
        Ok(serde_yaml::from_reader(f)?)
    }
    fn filename(name: &str) -> String {
        format!("{}.yaml", name)
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize, Deref)]
#[serde(rename_all = "kebab-case")]
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

// ---

#[repr(u8)]
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Enum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Element {
    Time,
    Level,
    Logger,
    Caller,
    Message,
    EqualSign,
    Brace,
    Quote,
    Delimiter,
    Comma,
    AtSign,
    Ellipsis,
    FieldKey,
    Null,
    Boolean,
    Number,
    String,
    Whitespace,
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

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Bold,
    Faint,
    Italic,
    Underline,
    SlowBlink,
    RapidBlink,
    Reverse,
    Conseal,
    CrossedOut,
}

// ---

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum Color {
    Plain(PlainColor),
    Palette(u8),
    RGB(RGB),
}

// ---

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlainColor {
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

#[derive(RustEmbed)]
#[folder = "etc/defaults/themes/"]
struct Asset;

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
mod tests {
    use super::*;

    #[test]
    fn test_rgb() {
        let a = RGB::from_str("#102030").unwrap();
        assert_eq!(a, RGB(16, 32, 48));
        let b: RGB = serde_json::from_str(r##""#102030""##).unwrap();
        assert_eq!(b, RGB(16, 32, 48));
    }
}
