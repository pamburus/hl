//! Theme configuration v0 support
//!
//! Handles legacy v0 theme loading with lenient deserialization and conversions.

// std imports
use std::{collections::HashMap, fmt};

// third-party imports
use derive_more::{Deref, DerefMut, IntoIterator};
use enumset::EnumSet;
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, Visitor},
};
use serde_value::Value;

// local imports
use crate::level::InfallibleLevel;

// relative imports
use super::{Color, Element, Mode, Tag, Version};

// ---

/// V0 theme deserialization target.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Theme {
    #[serde(deserialize_with = "enumset_serde::deserialize")]
    pub tags: EnumSet<Tag>,
    pub version: Version,
    pub elements: StylePack,
    pub levels: HashMap<InfallibleLevel, StylePack>,
    pub indicators: IndicatorPack,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            tags: EnumSet::new(),
            version: Version::default(),
            elements: StylePack::default(),
            levels: HashMap::new(),
            indicators: IndicatorPack::default(),
        }
    }
}

impl Theme {
    /// Validate v0 theme version before deserialization
    pub fn validate_version(version: &Version) -> Result<(), super::ThemeLoadError> {
        if *version != Version::V0_0 {
            return Err(super::ThemeLoadError::UnsupportedVersion {
                requested: *version,
                nearest: Version::V0_0,
                latest: Version::CURRENT,
            });
        }

        Ok(())
    }
}
// ---

/// Style represents an element's visual styling (v0 format - simple, no base).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct Style {
    pub modes: Vec<Mode>,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            modes: Vec::new(),
            foreground: None,
            background: None,
        }
    }
}

impl Default for &Style {
    fn default() -> Self {
        static DEFAULT: Style = Style::new();
        &DEFAULT
    }
}

// ---

/// Collection of Element->Style mappings with lenient deserialization.
#[derive(Clone, Debug, Default, Deref, DerefMut, IntoIterator)]
pub struct StylePack(HashMap<Element, Style>);

impl From<HashMap<Element, Style>> for StylePack {
    fn from(items: HashMap<Element, Style>) -> Self {
        Self(items)
    }
}

impl<'de> Deserialize<'de> for StylePack {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(StylePackDeserializeVisitor::new())
    }
}

struct StylePackDeserializeVisitor;

impl StylePackDeserializeVisitor {
    fn new() -> Self {
        Self
    }
}

impl<'de> Visitor<'de> for StylePackDeserializeVisitor {
    type Value = StylePack;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("style pack object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> std::result::Result<Self::Value, A::Error> {
        let mut items = HashMap::new();

        // Use Value as a generic "any value" type to handle unknown keys.
        // This is format-agnostic and works with all serde formats (YAML, TOML, JSON).
        // This allows us to:
        // 1. Deserialize the key as Value
        // 2. Try to convert it to Element (the expected key type)
        // 3. If conversion fails (unknown key), discard the value
        // This provides forward compatibility by silently ignoring unknown elements.
        while let Some(key) = access.next_key::<Value>()? {
            if let Ok(key) = Element::deserialize(key) {
                let value: Style = access.next_value()?;
                items.insert(key, value);
            } else {
                _ = access.next_value::<Value>()?;
            }
        }

        Ok(StylePack(items))
    }
}

// ---

/// IndicatorPack contains all indicator styles (v0 - simple, no generics)
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct IndicatorPack {
    pub sync: SyncIndicatorPack,
}

/// SyncIndicatorPack contains synchronization-related indicators
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SyncIndicatorPack {
    pub synced: Indicator,
    pub failed: Indicator,
}

/// Indicator represents a status indicator with inner/outer styles
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Indicator {
    #[serde(default)]
    pub outer: IndicatorStyle,
    #[serde(default)]
    pub inner: IndicatorStyle,
    #[serde(default)]
    pub text: String,
}

/// IndicatorStyle represents the styling of an indicator part
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct IndicatorStyle {
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub style: Style,
}
