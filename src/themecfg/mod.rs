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
    convert::TryFrom,
    fmt,
    hash::Hash,
    str::{self, FromStr},
};

// third-party imports
use enumset::{EnumSet, EnumSetType};
use serde::Deserialize;
use thiserror::Error;

// sub-modules
mod element;
mod error;
mod indicator;
mod mode;
mod raw;
mod theme;
mod v0;
mod v1;
mod version;

// Re-export commonly used types
pub use v1::{Role, StyleBase};
pub use {element::*, error::*, indicator::*, mode::*, raw::*, theme::*, version::*};
pub type StylePack = v1::StylePack<Element, Style>;
pub type StyleInventory = v1::StylePack<Role, Style>;
pub type RawStyle = v1::Style;

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
    pub modes: ModeSet,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            modes: ModeSet::new(),
            foreground: None,
            background: None,
        }
    }

    pub fn modes(self, modes: ModeSet) -> Self {
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

// ---

fn unhex(high: u8, low: u8) -> Option<u8> {
    let h = (high as char).to_digit(16)?;
    let l = (low as char).to_digit(16)?;
    Some((h as u8) << 4 | (l as u8))
}

#[cfg(test)]
pub mod testing {
    use super::{Result, Theme, theme};

    pub fn theme() -> Result<Theme> {
        theme::testing::theme()
    }
}

#[cfg(test)]
mod tests;
