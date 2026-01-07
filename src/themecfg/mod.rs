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
use std::{hash::Hash, str};

// third-party imports
use enumset::{EnumSet, EnumSetType};
use serde::Deserialize;
use thiserror::Error;

// sub-modules
mod color;
mod element;
mod error;
mod indicator;
mod level;
mod mode;
mod raw;
mod style;
mod theme;
mod v0;
mod v1;
mod version;

// Re-export commonly used types
pub use v1::{Role, StyleBase};
pub use {color::*, element::*, error::*, indicator::*, level::*, mode::*, raw::*, style::*, theme::*, version::*};
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

pub trait MergeOptions {
    type Output;

    fn merge_options(&self) -> Self::Output;
}

// ---

// Trait for types that support merging
pub trait Merge<T = Self> {
    fn merge(&mut self, other: T);
    fn merged(self, other: T) -> Self
    where
        Self: Sized,
    {
        let mut result = self;
        result.merge(other);
        result
    }
}

// Trait for types that support merging with options
pub trait MergeWithOptions<T = Self> {
    type Options;

    fn merge(&mut self, other: T, options: Self::Options);
    fn merged(self, other: T, options: Self::Options) -> Self
    where
        Self: Sized,
    {
        let mut result = self;
        result.merge(other, options);
        result
    }
}

// ---

#[cfg(test)]
pub mod testing {
    use super::{Result, Theme, theme};

    pub fn theme() -> Result<Theme> {
        theme::testing::theme()
    }
}

#[cfg(test)]
pub(crate) mod tests;
