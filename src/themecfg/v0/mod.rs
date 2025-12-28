//! Theme configuration v0 support
//!
//! Handles legacy v0 theme loading with lenient deserialization and conversions.

// std imports
use std::{collections::HashMap, fmt, hash::Hash, sync::LazyLock};

// third-party imports
use derive_more::{Deref, DerefMut, IntoIterator};
use enum_map::Enum;
use enumset::EnumSet;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};
use serde_value::Value;
use strum::{EnumIter, IntoEnumIterator};

// local imports
use crate::level::InfallibleLevel;

// Re-exports from parent module (common types)
pub use super::{Color, MergeFlag, MergeFlags, Mode, PlainColor, RGB, Tag, ThemeVersion};

// ---
// v0 does not have Role - that's a v1 feature for semantic styling

// ---

/// Element represents a UI element that can be styled (v0).
#[repr(u8)]
#[derive(Debug, Default, Hash, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, Enum, Deserialize, Serialize, EnumIter)]
#[serde(rename_all = "kebab-case")]
pub enum Element {
    #[default]
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
        self.outer().is_some()
    }

    pub fn outer(&self) -> Option<Self> {
        match self {
            Self::InputNumber => Some(Self::Input),
            Self::InputName => Some(Self::Input),
            Self::InputNumberInner => Some(Self::InputNumber),
            Self::InputNameInner => Some(Self::InputName),
            Self::LevelInner => Some(Self::Level),
            Self::LoggerInner => Some(Self::Logger),
            Self::CallerInner => Some(Self::Caller),
            _ => None,
        }
    }

    pub fn nested() -> &'static [(Self, Self)] {
        static PAIRS: LazyLock<Vec<(Element, Element)>> = LazyLock::new(|| {
            Element::iter()
                .filter_map(|element| element.outer().map(|parent| (parent, element)))
                .collect()
        });
        &PAIRS
    }
}

// ---

// v0 uses simple Vec<Mode> (no ModeSetDiff like v1)

// ---

// v0 does not have StyleBase - that's a v1 feature

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

// ---

/// V0 theme deserialization target.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Theme {
    #[serde(deserialize_with = "enumset_serde::deserialize")]
    pub tags: EnumSet<Tag>,
    pub version: ThemeVersion,
    // v0 does not have styles section - only elements
    pub elements: StylePack,
    pub levels: HashMap<InfallibleLevel, StylePack>,
    pub indicators: IndicatorPack,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            tags: EnumSet::new(),
            version: ThemeVersion::default(),
            elements: StylePack::default(),
            levels: HashMap::new(),
            indicators: IndicatorPack::default(),
        }
    }
}

impl Theme {
    /// Validate v0 theme version before deserialization
    ///
    /// V0 themes must be exactly version 0.0 (or default/unspecified)
    /// This is called before deserialization to provide better error messages
    pub fn validate_version(version: &ThemeVersion) -> Result<(), super::ThemeLoadError> {
        // Default version (0.0) is always acceptable for v0
        if *version == ThemeVersion::default() {
            return Ok(());
        }

        // V0 themes must be exactly version 0.0
        if *version != ThemeVersion::V0_0 {
            return Err(super::ThemeLoadError::UnsupportedVersion {
                requested: *version,
                supported: ThemeVersion::V0_0,
            });
        }

        Ok(())
    }
}
