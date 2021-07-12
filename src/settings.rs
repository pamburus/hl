// std imports
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{self, Write};
use std::include_str;
use std::str::{self, FromStr};

// third-party imports
use chrono_tz::Tz;
use config::{Config, File, FileFormat};
use derive_deref::Deref;
use platform_dirs::AppDirs;
use serde::Deserialize;

// local imports
use crate::error::Error;
use crate::types::Level;

// ---

static DEFAULT_SETTINGS: &str = include_str!("../etc/defaults/config.yaml");

// ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    pub fields: Fields,
    pub concurrency: Option<usize>,
    pub time_format: String,
    pub time_zone: Tz,
    pub theme: String,
    pub themes: HashMap<String, Theme>,
}

impl Settings {
    pub fn load(app_dirs: &AppDirs) -> Result<Self, Error> {
        let mut s = Config::default();
        let filename = app_dirs.config_dir.join("config.yaml");

        s.merge(File::from_str(DEFAULT_SETTINGS, FileFormat::Yaml))?;
        s.merge(File::with_name(&filename.to_string_lossy()).required(false))?;

        Ok(s.try_into()?)
    }
}

// ---

#[derive(Debug, Deserialize)]
pub struct Fields {
    pub predefined: PrefedinedFields,
    pub ignore: Vec<String>,
    pub hide: Vec<String>,
}

// ---

#[derive(Debug, Deserialize)]
pub struct PrefedinedFields {
    pub time: TimeField,
    pub level: LevelField,
    pub message: MessageField,
    pub logger: LoggerField,
    pub caller: CallerField,
}

// ---

#[derive(Debug, Deserialize, Deref)]
pub struct TimeField(pub Field);

// ---

#[derive(Debug, Deserialize)]
pub struct LevelField {
    pub variants: Vec<LevelFieldVariant>,
}

// ---

#[derive(Debug, Deserialize)]
pub struct LevelFieldVariant {
    pub names: Vec<String>,
    pub values: HashMap<Level, Vec<String>>,
}

// ---

#[derive(Debug, Deserialize, Deref)]
pub struct MessageField(Field);

// ---

#[derive(Debug, Deserialize, Deref)]
pub struct LoggerField(Field);

// ---

#[derive(Debug, Deserialize, Deref)]
pub struct CallerField(Field);

// ---

#[derive(Debug, Deserialize)]
pub struct Field {
    pub names: Vec<String>,
}

// ---

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub default: StylePack<Style>,
    pub levels: HashMap<Level, StylePack<Option<Style>>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct StylePack<Style> {
    pub time: Style,
    pub level: Style,
    pub logger: Style,
    pub caller: Style,
    pub message: Style,
    pub equal_sign: Style,
    pub brace: Style,
    pub quote: Style,
    pub delimiter: Style,
    pub comma: Style,
    pub at_sign: Style,
    pub ellipsis: Style,
    pub field_key: Style,
    pub null: Style,
    pub boolean: Style,
    pub number: Style,
    pub string: Style,
    pub whitespace: Style,
}

impl StylePack<Style> {
    pub fn merge(&mut self, patch: &StylePack<Option<Style>>) {
        Self::patch(&mut self.time, &patch.time);
        Self::patch(&mut self.level, &patch.level);
        Self::patch(&mut self.logger, &patch.logger);
        Self::patch(&mut self.caller, &patch.caller);
        Self::patch(&mut self.message, &patch.message);
        Self::patch(&mut self.equal_sign, &patch.equal_sign);
        Self::patch(&mut self.brace, &patch.brace);
        Self::patch(&mut self.quote, &patch.quote);
        Self::patch(&mut self.delimiter, &patch.delimiter);
        Self::patch(&mut self.comma, &patch.comma);
        Self::patch(&mut self.at_sign, &patch.at_sign);
        Self::patch(&mut self.ellipsis, &patch.ellipsis);
        Self::patch(&mut self.field_key, &patch.field_key);
        Self::patch(&mut self.null, &patch.null);
        Self::patch(&mut self.boolean, &patch.boolean);
        Self::patch(&mut self.number, &patch.number);
        Self::patch(&mut self.string, &patch.string);
        Self::patch(&mut self.whitespace, &patch.whitespace);
    }

    pub fn merged(mut self, patch: &StylePack<Option<Style>>) -> Self {
        self.merge(patch);
        self
    }

    fn patch<T: Clone>(value: &mut T, patch: &Option<T>) {
        if let Some(patch) = patch {
            *value = patch.clone();
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct Style {
    pub modes: Vec<Mode>,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

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

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum Color {
    Plain(PlainColor),
    Palette(u8),
    RGB(RGB),
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
#[serde(try_from = "String")]
pub struct RGB(pub u8, pub u8, pub u8);

impl FromStr for RGB {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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

    fn try_from(s: String) -> Result<Self, Self::Error> {
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
