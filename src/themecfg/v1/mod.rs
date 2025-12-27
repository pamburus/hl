//! Theme configuration v1 support
//!
//! This module contains v1-specific theme loading logic, including:
//! - Strict deserialization (fails on unknown keys/values)
//! - Type reuse from v0 for unchanged types
//! - V1-specific validation

// std imports
use std::collections::HashMap;

// third-party imports
use enumset::EnumSet;
use serde::{Deserialize, Serialize};

// local imports
use crate::level::InfallibleLevel;

// Re-export Element from v0 (unchanged in v1)
pub use super::v0::Element;

// Import v0 module for conversion from v0 to v1
use super::v0;

// Import traits from parent

// Re-export common types from parent module
pub use super::{
    Color, MergeFlag, MergeFlags, Mode, ModeDiff, ModeDiffAction, ModeSet, ModeSetDiff, PlainColor, RGB, Tag,
    ThemeVersion,
};

// ---

/// Role represents a semantic style role (v1 feature)
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    #[default]
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

// ModeSetDiff, ModeDiff, ModeDiffAction are now imported from parent module

/// StyleBase represents base styles for inheritance (v1 feature)
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

/// Style with v1 features (base, ModeSetDiff)
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

// ---

/// StylePack for v1 - strict deserialization, generic over key and style types
#[derive(Clone, Debug, Default)]
pub struct StylePack<K, S = Style>(pub HashMap<K, S>);

impl<'de, K, S> Deserialize<'de> for StylePack<K, S>
where
    K: Deserialize<'de> + std::cmp::Eq + std::hash::Hash,
    S: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // For v1, we use standard HashMap deserialization which fails on unknown enum variants
        let map = HashMap::<K, S>::deserialize(deserializer)?;
        Ok(StylePack(map))
    }
}

impl<K, S> StylePack<K, S> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, key: &K) -> Option<&S>
    where
        K: std::cmp::Eq + std::hash::Hash,
    {
        self.0.get(key)
    }

    pub fn items(&self) -> impl Iterator<Item = (&K, &S)> {
        self.0.iter()
    }
}

// ---

/// RawTheme is the v1 theme deserialization target
/// It uses strict deserialization and fails on unknown fields
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RawTheme {
    #[serde(deserialize_with = "enumset_serde::deserialize")]
    pub tags: EnumSet<Tag>,
    pub version: ThemeVersion,
    pub styles: StylePack<Role>,
    pub elements: StylePack<Element>,
    pub levels: HashMap<InfallibleLevel, StylePack<Element>>,
    pub indicators: IndicatorPack,
}

impl Default for RawTheme {
    fn default() -> Self {
        Self {
            tags: EnumSet::new(),
            version: ThemeVersion::default(),
            styles: StylePack::default(),
            elements: StylePack::default(),
            levels: HashMap::new(),
            indicators: IndicatorPack::default(),
        }
    }
}

// ---

// ---

/// Indicator types for v1 (generic over style type)
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct IndicatorPack<S = Style> {
    pub sync: SyncIndicatorPack<S>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct SyncIndicatorPack<S = Style> {
    pub synced: Indicator<S>,
    #[serde(rename = "sync-failed")]
    pub failed: Indicator<S>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct Indicator<S = Style> {
    #[serde(default)]
    pub outer: IndicatorStyle<S>,
    #[serde(default)]
    pub inner: IndicatorStyle<S>,
    #[serde(default)]
    pub text: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct IndicatorStyle<S = Style> {
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub style: S,
}

// ---

/// Convert v0::RawTheme to v1::RawTheme
impl From<v0::RawTheme> for RawTheme {
    fn from(theme: v0::RawTheme) -> Self {
        // Convert v0 elements to v1 format
        let mut elements = HashMap::new();
        for (element, style) in theme.elements.0 {
            elements.insert(element, style.into());
        }

        // Convert v0 levels to v1 format
        let mut levels = HashMap::new();
        for (level, pack) in theme.levels {
            let mut pack_map = HashMap::new();
            for (element, style) in pack.0 {
                pack_map.insert(element, style.into());
            }
            levels.insert(level, StylePack(pack_map));
        }

        // Convert v0 indicators to v1 format
        let indicators = theme.indicators.into();

        // Deduce styles from elements for v0 themes
        let styles = deduce_styles_from_elements(&elements);

        Self {
            tags: theme.tags,
            version: theme.version,
            styles: StylePack(styles),
            elements: StylePack(elements),
            levels,
            indicators,
        }
    }
}

/// Convert v0::Style (Vec<Mode>) to v1::Style (ModeSetDiff)
impl From<v0::Style> for Style {
    fn from(style: v0::Style) -> Self {
        let modes = style.modes.into_iter().collect::<ModeSet>().into();
        Self {
            base: StyleBase::default(),
            modes,
            foreground: style.foreground,
            background: style.background,
        }
    }
}

/// Convert v0 indicators to v1 indicators
impl From<v0::IndicatorPack> for IndicatorPack<Style> {
    fn from(indicators: v0::IndicatorPack) -> Self {
        Self {
            sync: SyncIndicatorPack {
                synced: Indicator {
                    outer: IndicatorStyle {
                        prefix: indicators.sync.synced.outer.prefix,
                        suffix: indicators.sync.synced.outer.suffix,
                        style: indicators.sync.synced.outer.style.into(),
                    },
                    inner: IndicatorStyle {
                        prefix: indicators.sync.synced.inner.prefix,
                        suffix: indicators.sync.synced.inner.suffix,
                        style: indicators.sync.synced.inner.style.into(),
                    },
                    text: indicators.sync.synced.text,
                },
                failed: Indicator {
                    outer: IndicatorStyle {
                        prefix: indicators.sync.failed.outer.prefix,
                        suffix: indicators.sync.failed.outer.suffix,
                        style: indicators.sync.failed.outer.style.into(),
                    },
                    inner: IndicatorStyle {
                        prefix: indicators.sync.failed.inner.prefix,
                        suffix: indicators.sync.failed.inner.suffix,
                        style: indicators.sync.failed.inner.style.into(),
                    },
                    text: indicators.sync.failed.text,
                },
            },
        }
    }
}

/// Deduce styles from elements for v0 themes
fn deduce_styles_from_elements(elements: &HashMap<Element, Style>) -> HashMap<Role, Style> {
    let mut styles = HashMap::new();

    let element_to_role = [
        (Element::String, Role::Primary),
        (Element::Time, Role::Secondary),
        (Element::Message, Role::Strong),
        (Element::Key, Role::Accent),
        (Element::Array, Role::Syntax),
    ];

    for (element, role) in element_to_role {
        if let Some(element_style) = elements.get(&element) {
            styles.insert(role, element_style.clone());
        }
    }

    styles
}
