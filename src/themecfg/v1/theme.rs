//! Theme configuration v1 format support.
//!
//! Implements strict, semantic theme loading with role-based inheritance.
//! Supports `$schema`, mode diffs, and deep inheritance chains (up to 64 levels).

// std imports
use std::collections::HashMap;

// third-party imports
use enumset::EnumSet;
use serde::Deserialize;

// local imports
use crate::level::{InfallibleLevel, Level};

// relative imports
use super::{
    Element, IndicatorPack, Merge, MergeFlag, MergeFlags, MergeOptions, MergeWithOptions, ResolvedIndicatorPack,
    ResolvedTheme, Result, Role, Style, StyleInventory, StylePack, StyleResolveError, Tag, ThemeLoadError, Version, v0,
};

// ---

/// V1 theme definition (unresolved).
///
/// Contains unresolved style definitions that may reference roles and use inheritance.
/// Uses strict deserialization (fails on unknown fields) and optionally accepts `$schema`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    /// Optional JSON/YAML schema reference for IDE/validator support.
    ///
    /// This field is accepted in theme files but ignored during processing.
    /// It enables IDE features like autocomplete and validation when editing themes.
    #[serde(rename = "$schema")]
    #[serde(skip_serializing)]
    pub schema: Option<String>,
    #[serde(deserialize_with = "enumset_serde::deserialize")]
    pub tags: EnumSet<Tag>,
    pub version: Version,
    pub styles: StylePack<Role>,
    pub elements: StylePack<Element>,
    pub levels: HashMap<Level, StylePack<Element>>,
    pub indicators: IndicatorPack,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            schema: None,
            tags: EnumSet::new(),
            version: Version::default(),
            styles: StylePack::default(),
            elements: StylePack::default(),
            levels: HashMap::new(),
            indicators: IndicatorPack::default(),
        }
    }
}

impl Theme {
    /// Resolves all styles in this theme and returns a resolved Theme.
    ///
    /// This method:
    /// 1. Resolves the role-based styles inventory
    /// 2. Applies the inventory to element-based styles
    /// 3. Handles outer-inner element inheritance
    /// 4. Processes level-specific element overrides
    /// 5. Resolves indicator styles
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Style recursion limit is exceeded
    pub fn resolve(self) -> Result<ResolvedTheme, StyleResolveError> {
        let flags = self.merge_options();

        // Step 1: Resolve the role-based styles inventory
        let inventory = self.styles.resolved(flags)?;

        // Step 2: Resolve base element styles
        let elements = self.elements.resolved(&inventory, flags);

        // Step 3: Resolve level-specific element styles
        let mut levels = HashMap::new();
        for (level, pack) in &self.levels {
            // Merge base elements with level-specific elements
            let pack = self
                .elements
                .clone()
                .merged(pack, flags - MergeFlag::ReplaceHierarchies)
                .resolved(&inventory, flags);
            levels.insert(*level, pack);
        }

        // Step 4: Resolve indicator styles
        let indicators = Self::resolve_indicators(&self.indicators, &inventory, flags);

        Ok(ResolvedTheme {
            tags: self.tags,
            version: self.version,
            elements,
            levels,
            indicators,
        })
    }

    fn resolve_indicators(
        indicators: &IndicatorPack<Style>,
        inventory: &StyleInventory,
        flags: MergeFlags,
    ) -> ResolvedIndicatorPack {
        indicators.clone().resolve(|style| style.resolve(inventory, flags))
    }

    /// Validate v1 theme version before deserialization
    ///
    /// V1 themes must be compatible with the current version
    /// This is called before deserialization to provide better error messages
    pub fn validate_version(version: &Version) -> Result<(), ThemeLoadError> {
        const CURRENT: Version = Version::V1;

        // Check if version is compatible with current supported version
        if !version.is_compatible_with(&Version::CURRENT) {
            return Err(ThemeLoadError::UnsupportedVersion {
                requested: *version,
                nearest: CURRENT,
                latest: Version::CURRENT,
            });
        }

        Ok(())
    }
}

impl MergeOptions for Theme {
    type Output = MergeFlags;

    fn merge_options(&self) -> Self::Output {
        self.version.merge_options()
    }
}

impl Merge for Theme {
    fn merge(&mut self, other: Self) {
        let flags = other.merge_options();
        self.version = other.version;
        self.styles.merge(other.styles);

        // Apply blocking rules only for version 0 themes (backward compatibility)
        if flags.contains(MergeFlag::ReplaceHierarchies) {
            // Apply blocking rule: remove all elements from self that have any ancestor
            // element defined in other.elements (including direct parent and all grand-parents)
            self.elements.retain(|element, _| {
                // Check if any ancestor of this element is defined in other.elements
                let mut current = *element;
                while let Some(parent) = current.outer() {
                    if other.elements.contains_key(&parent) {
                        return false; // This element should be removed
                    }
                    current = parent;
                }
                true // Keep this element
            });
            // Block entire level sections if child theme defines any element for that level
            for level in other.levels.keys() {
                self.levels.remove(level);
            }
        }

        // For both v0 and v1, elements defined in child theme replace elements from parent theme
        // Property-level merge happens later when merging elements with per-level styles
        self.elements.extend(other.elements);

        // For both v0 and v1, level-specific elements defined in child theme replace from parent
        for (level, pack) in other.levels {
            self.levels
                .entry(level)
                .and_modify(|existing| existing.extend(pack.clone()))
                .or_insert(pack);
        }

        self.tags = other.tags;
        self.indicators.merge(other.indicators, flags);
    }
}

impl<T> Merge<T> for Theme
where
    T: IntoIterator<Item = Theme>,
{
    fn merge(&mut self, other: T) {
        for theme in other {
            self.merge(theme);
        }
    }
}

/// Convert v0::Theme to v1::Theme
impl From<v0::Theme> for Theme {
    fn from(theme: v0::Theme) -> Self {
        // Convert v0 elements to v1 format
        let elements: HashMap<Element, Style> = theme
            .elements
            .iter()
            .map(|(e, style)| (*e, style.clone().into()))
            .collect();

        // Convert v0 levels to v1 format
        let mut levels = HashMap::new();
        for (level, pack) in theme.levels {
            // Only convert valid levels - v1 is strict, invalid levels are dropped
            if let InfallibleLevel::Valid(level) = level {
                let pack: HashMap<Element, Style> = pack.iter().map(|(e, style)| (*e, style.clone().into())).collect();
                levels.insert(level, StylePack::new(pack));
            }
        }

        // Convert v0 indicators to v1 format
        let indicators = theme.indicators.into();

        // Deduce styles from elements for v0 themes
        let styles = deduce_styles_from_elements(&elements);

        Self {
            schema: None,
            tags: theme.tags,
            version: theme.version,
            styles: StylePack::new(styles),
            elements: StylePack::new(elements),
            levels,
            indicators,
        }
    }
}

// ---

/// Deduce styles from elements for v0 themes
fn deduce_styles_from_elements(elements: &HashMap<Element, Style>) -> HashMap<Role, Style> {
    let mut styles = HashMap::new();

    const MAPPING: &[(Element, Role)] = &[
        (Element::String, Role::Primary),
        (Element::Time, Role::Secondary),
        (Element::Message, Role::Strong),
        (Element::Key, Role::Accent),
        (Element::Array, Role::Syntax),
    ];

    for &(element, role) in MAPPING {
        if let Some(style) = elements.get(&element) {
            styles.insert(role, style.clone());
        }
    }

    styles
}
