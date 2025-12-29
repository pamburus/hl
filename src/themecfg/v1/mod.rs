//! Theme configuration v1 format support.
//!
//! Implements strict, semantic theme loading with role-based inheritance.
//! Supports `$schema`, mode diffs, and deep inheritance chains (up to 64 levels).

// std imports
use std::collections::HashMap;

// third-party imports
use derive_more::{Deref, DerefMut, IntoIterator};
use enumset::EnumSet;
use serde::{Deserialize, Serialize};

// local imports
use crate::level::{InfallibleLevel, Level};

// relative imports
use super::{
    Color, Element, MergeFlag, MergeFlags, Mode, ModeSet, ModeSetDiff, Result, StyleInventory, Tag, ThemeLoadError,
    ThemeVersion, v0,
};

// ---

// Import resolved types and traits from parent (output types)
use super::Merge;

// Import the resolved Style from parent (was ResolvedStyle)
use super::Style as ResolvedStyle;

// Constants

/// Maximum depth for role-to-role style inheritance chains.
///
/// Limits recursion depth to 64 (FR-046) to prevent infinite loops and stack overflow.
/// Circular references will trigger this limit (FR-047).
const RECURSION_LIMIT: usize = 64;

// ---

/// Semantic style role for theme inheritance (v1 feature).
///
/// Defines reusable styles (e.g., primary, warning) that elements can inherit from.
/// Roles are serialized in kebab-case (e.g., `AccentSecondary` -> `accent-secondary`).
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

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_plain::to_string(self) {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "{:?}", self),
        }
    }
}

// ---

/// Base style inheritance specification (v1 feature).
///
/// Specifies generic roles to inherit from (single or multiple).
/// Merge order is left-to-right (later roles override earlier ones).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct StyleBase(Vec<Role>);

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

impl std::fmt::Display for StyleBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, role) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", role)?;
        }
        write!(f, "]")
    }
}

// ---

// ---

// Conversion helpers for ModeSetDiff (v1 unresolved mode representation)
impl From<ModeSet> for ModeSetDiff {
    fn from(modes: ModeSet) -> Self {
        Self {
            adds: modes,
            removes: ModeSet::new(),
        }
    }
}

impl From<Mode> for ModeSetDiff {
    fn from(mode: Mode) -> Self {
        Self {
            adds: mode.into(),
            removes: ModeSet::new(),
        }
    }
}

// ---

/// Style with v1 features (base, ModeSetDiff).
/// This is the unresolved style type used in v1 themes.
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

impl Style {
    pub const fn new() -> Self {
        Self {
            base: StyleBase(Vec::new()),
            modes: ModeSetDiff::new(),
            foreground: None,
            background: None,
        }
    }

    pub fn base(self, base: impl Into<StyleBase>) -> Self {
        Self {
            base: base.into(),
            ..self
        }
    }

    pub fn modes(self, modes: ModeSetDiff) -> Self {
        Self { modes, ..self }
    }

    pub fn background(self, background: Option<Color>) -> Self {
        Self { background, ..self }
    }

    pub fn foreground(self, foreground: Option<Color>) -> Self {
        Self { foreground, ..self }
    }

    pub fn merged(mut self, other: &Self, flags: MergeFlags) -> Self {
        if !other.base.is_empty() {
            self.base = other.base.clone();
        }
        if flags.contains(MergeFlag::ReplaceModes) {
            self.modes = other.modes;
        } else {
            self.modes += other.modes;
        }
        if let Some(color) = other.foreground {
            self.foreground = Some(color);
        }
        if let Some(color) = other.background {
            self.background = Some(color);
        }
        self
    }

    pub fn resolve(&self, inventory: &StyleInventory, flags: MergeFlags) -> ResolvedStyle {
        Self::resolve_with(&self.base, self, flags, |role| {
            inventory.get(role).cloned().unwrap_or_default()
        })
    }

    pub fn resolve_with<F>(bases: &StyleBase, style: &Style, flags: MergeFlags, mut resolve_role: F) -> ResolvedStyle
    where
        F: FnMut(&Role) -> ResolvedStyle,
    {
        if bases.is_empty() {
            return style.as_resolved();
        }

        // Resolve multiple bases: merge left to right, then apply style on top
        let mut result = ResolvedStyle::default();
        for role in bases.iter() {
            result = result.merged(&resolve_role(role), flags);
        }
        // When applying the style's own properties on top of the resolved base,
        // we should NOT use ReplaceModes - the style's properties should be merged additively
        // with the base, not replace them. ReplaceModes is only for theme-level merging.
        result.merged(style, flags - MergeFlag::ReplaceModes)
    }

    pub fn as_resolved(&self) -> ResolvedStyle {
        ResolvedStyle {
            modes: self.modes.adds,
            foreground: self.foreground,
            background: self.background,
        }
    }
}

impl Default for &Style {
    fn default() -> Self {
        static DEFAULT: Style = Style::new();
        &DEFAULT
    }
}

impl Merge<&Style> for Style {
    fn merge(&mut self, other: &Style, flags: MergeFlags) {
        *self = self.clone().merged(other, flags);
    }
}

impl From<Role> for Style {
    fn from(role: Role) -> Self {
        Self {
            base: StyleBase::from(role),
            ..Default::default()
        }
    }
}

impl From<Vec<Role>> for Style {
    fn from(roles: Vec<Role>) -> Self {
        Self {
            base: StyleBase::from(roles),
            ..Default::default()
        }
    }
}

impl From<ResolvedStyle> for Style {
    fn from(body: ResolvedStyle) -> Self {
        Self {
            base: StyleBase::default(),
            modes: body.modes.into(),
            foreground: body.foreground,
            background: body.background,
        }
    }
}

// Merge implementations for ResolvedStyle (used during resolution)
// These live in v1 module because they're part of the resolution logic

impl Merge<&ResolvedStyle> for ResolvedStyle {
    fn merge(&mut self, other: &ResolvedStyle, flags: MergeFlags) {
        // For resolved styles, merge mode diffs
        if flags.contains(MergeFlag::ReplaceModes) {
            self.modes = other.modes;
        } else {
            self.modes |= other.modes;
        }
        if let Some(color) = other.foreground {
            self.foreground = Some(color);
        }
        if let Some(color) = other.background {
            self.background = Some(color);
        }
    }
}

impl Merge<&Style> for ResolvedStyle {
    fn merge(&mut self, other: &Style, flags: MergeFlags) {
        // When merging an unresolved style (v1::Style) into a resolved style
        if flags.contains(MergeFlag::ReplaceModes) {
            self.modes = other.modes.adds;
        } else {
            self.modes |= other.modes.adds;
            self.modes -= other.modes.removes;
        }
        if let Some(color) = other.foreground {
            self.foreground = Some(color);
        }
        if let Some(color) = other.background {
            self.background = Some(color);
        }
    }
}

// ---

/// StylePack for v1 - strict deserialization, generic over key and style types
#[derive(Clone, Debug, Deref, DerefMut, IntoIterator)]
pub struct StylePack<K, S = Style>(HashMap<K, S>);

impl<K, S> Default for StylePack<K, S> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

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

impl<K, S> StylePack<K, S>
where
    K: std::cmp::Eq + std::hash::Hash,
{
    pub fn new(items: HashMap<K, S>) -> Self {
        Self(items)
    }
}

impl<S> StylePack<Role, S> {
    pub fn merge(&mut self, patch: Self) {
        self.0.extend(patch.0);
    }

    pub fn merged(mut self, patch: Self) -> Self {
        self.merge(patch);
        self
    }
}

impl<S> StylePack<Element, S> {
    pub fn merge(&mut self, patch: Self, flags: MergeFlags)
    where
        S: Clone + for<'a> Merge<&'a S>,
    {
        if flags.contains(MergeFlag::ReplaceHierarchies) {
            for (parent, child) in Element::nested() {
                if patch.0.contains_key(child) {
                    self.0.remove(parent);
                }
            }
        }

        if flags.contains(MergeFlag::ReplaceElements) {
            self.0.extend(patch.0);
            return;
        }

        for (key, patch) in patch.0 {
            self.0
                .entry(key)
                .and_modify(|v| *v = v.clone().merged(&patch, flags))
                .or_insert(patch);
        }
    }
}

impl StylePack<Element, Style> {
    pub fn resolved(&self, inventory: &StyleInventory, flags: MergeFlags) -> StylePack<Element, ResolvedStyle> {
        self.clone().complete_hierarchy(flags).resolve_styles(inventory, flags)
    }

    /// Completes the element hierarchy by applying parent→inner and boolean variant inheritance.
    /// This is a merge operation that must happen before style resolution.
    ///
    /// This method:
    /// 1. Merges parent styles into inner elements (unless ReplaceElements flag is set)
    /// 2. Adds inherited inner elements that weren't explicitly defined
    /// 3. Handles boolean variant inheritance from base Boolean element
    ///
    /// Per FR-041, this merging of unresolved styles must occur before resolving `base` references.
    pub fn complete_hierarchy(mut self, flags: MergeFlags) -> Self {
        // Step 1: Merge parent→inner where inner is explicitly defined (v1 only)
        // For v0 (ReplaceElements), inner elements replace parent completely
        if !flags.contains(MergeFlag::ReplaceElements) {
            // V1: Merge parent into each explicitly-defined inner element
            for (element, style) in self.clone() {
                if let Some(outer) = element.outer() {
                    if let Some(outer) = self.0.get(&outer) {
                        // Merge unresolved parent and inner first (per FR-041)
                        self.0.insert(element, outer.clone().merged(&style, flags));
                    }
                }
            }
        }

        // Step 2: Add inherited inner elements that weren't explicitly defined
        // Use canonical pairs from Element::nested() for single source of truth (FR-015a)
        for &(outer, inner) in Element::nested() {
            if let Some(outer) = self.0.get(&outer).cloned() {
                self.0.entry(inner).or_insert_with(|| outer);
            }
        }

        // Step 3: Handle boolean variants inheriting from base boolean
        if let Some(base) = self.0.get(&Element::Boolean).cloned() {
            for variant in [Element::BooleanTrue, Element::BooleanFalse] {
                self.0
                    .entry(variant)
                    .and_modify(|style| *style = base.clone().merged(style, flags))
                    .or_insert_with(|| base.clone());
            }
        }

        self
    }

    /// Resolves all styles in this pack by converting `base` references to actual styles.
    /// This is a pure resolution operation that should be called after all merging is complete.
    ///
    /// Per FR-041, all merging (including hierarchy completion) must happen before resolution.
    pub fn resolve_styles(&self, inventory: &StyleInventory, flags: MergeFlags) -> super::StylePack {
        let items: HashMap<Element, ResolvedStyle> = self
            .iter()
            .map(|(&element, style)| (element, style.resolve(inventory, flags)))
            .collect();
        super::StylePack::new(items)
    }
}

impl Merge<&StylePack<Element, Style>> for StylePack<Element, Style> {
    fn merge(&mut self, other: &StylePack<Element, Style>, flags: MergeFlags) {
        Self::merge(self, other.clone(), flags);
    }
}

impl Merge<StylePack<Element, Style>> for StylePack<Element, Style> {
    fn merge(&mut self, other: StylePack<Element, Style>, flags: MergeFlags) {
        Self::merge(self, other, flags);
    }
}

impl StylePack<Role, Style> {
    pub fn resolved(&self, flags: MergeFlags) -> Result<StyleInventory, ThemeLoadError> {
        let mut resolver = StyleResolver::new(self, flags);
        let items: HashMap<Role, ResolvedStyle> = self
            .keys()
            .map(|k| Ok((*k, resolver.resolve(k)?)))
            .collect::<Result<HashMap<Role, ResolvedStyle>, ThemeLoadError>>()?;
        Ok(StyleInventory::new(items))
    }
}

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
    pub version: ThemeVersion,
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
            version: ThemeVersion::default(),
            styles: StylePack::default(),
            elements: StylePack::default(),
            levels: HashMap::new(),
            indicators: IndicatorPack::default(),
        }
    }
}

impl Theme {
    pub fn merge(&mut self, other: Self) {
        let flags = other.merge_flags();
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

    pub fn merged(mut self, other: Self) -> Self {
        self.merge(other);
        self
    }

    /// Resolves all styles in this theme and returns a resolved Theme.
    ///
    /// This method:
    /// 1. Resolves the role-based styles inventory
    /// 2. Applies the inventory to element-based styles
    /// 3. Handles parent-inner element inheritance
    /// 4. Processes level-specific element overrides
    /// 5. Resolves indicator styles
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Style recursion limit is exceeded
    /// - Any other style resolution error occurs
    pub fn resolve(self) -> Result<super::Theme, ThemeLoadError> {
        let flags = self.merge_flags();

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

        Ok(super::Theme {
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
    ) -> super::IndicatorPack {
        indicators.clone().resolve(|style| style.resolve(inventory, flags))
    }

    pub fn merge_flags(&self) -> MergeFlags {
        match self.version {
            ThemeVersion { major: 0, .. } => {
                MergeFlag::ReplaceElements | MergeFlag::ReplaceHierarchies | MergeFlag::ReplaceModes
            }
            _ => MergeFlags::new(),
        }
    }

    /// Validate v1 theme version before deserialization
    ///
    /// V1 themes must be compatible with the current version
    /// This is called before deserialization to provide better error messages
    pub fn validate_version(version: &ThemeVersion) -> Result<(), super::ThemeLoadError> {
        const CURRENT: ThemeVersion = ThemeVersion::V1;

        // Check if version is compatible with current supported version
        if !version.is_compatible_with(&ThemeVersion::CURRENT) {
            return Err(super::ThemeLoadError::UnsupportedVersion {
                requested: *version,
                nearest: CURRENT,
                latest: ThemeVersion::CURRENT,
            });
        }

        Ok(())
    }
}

// ---

/// Helper for resolving role-based styles with caching and recursion protection.
///
/// Resolves role-to-role inheritance chains while enforcing dependency limits (64 levels)
/// to prevent stack overflow and infinite loops from circular references.
pub struct StyleResolver<'a> {
    inventory: &'a StylePack<Role, Style>,
    flags: MergeFlags,
    cache: HashMap<Role, ResolvedStyle>,
    depth: usize,
}

impl<'a> StyleResolver<'a> {
    fn new(inventory: &'a StylePack<Role, Style>, flags: MergeFlags) -> Self {
        Self {
            inventory,
            flags,
            cache: HashMap::new(),
            depth: 0,
        }
    }

    fn resolve(&mut self, role: &Role) -> Result<ResolvedStyle, ThemeLoadError> {
        if let Some(resolved) = self.cache.get(role) {
            return Ok(resolved.clone());
        }

        let style = self.inventory.get(role).unwrap_or_default();

        self.depth += 1;
        let resolved = self.resolve_style(style, role)?;
        self.depth -= 1;

        self.cache.insert(*role, resolved.clone());

        Ok(resolved)
    }

    fn resolve_style(&mut self, style: &Style, role: &Role) -> Result<ResolvedStyle, ThemeLoadError> {
        // If no explicit base, default to inheriting from Default role (except for Default itself)
        let base = if style.base.is_empty() {
            if *role != Role::Default {
                StyleBase::from(Role::Default)
            } else {
                StyleBase::default()
            }
        } else {
            style.base.clone()
        };

        if !base.is_empty() && self.depth >= RECURSION_LIMIT {
            return Err(ThemeLoadError::StyleRecursionLimitExceeded {
                role: *role,
                base,
                limit: RECURSION_LIMIT,
            });
        }

        let mut result = ResolvedStyle::default();
        for base_role in base.0 {
            let base_resolved = self.resolve(&base_role)?;
            result = result.merged(&base_resolved, self.flags);
        }

        Ok(result.merged(&style.as_resolved(), self.flags))
    }
}

// ---

/// Indicator types for v1 (generic over style type)
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct IndicatorPack<S = Style> {
    pub sync: SyncIndicatorPack<S>,
}

impl<S: Clone> IndicatorPack<S> {
    pub fn merge(&mut self, other: Self, flags: MergeFlags)
    where
        SyncIndicatorPack<S>: Merge,
    {
        self.sync.merge(other.sync, flags);
    }

    pub fn merged(mut self, other: Self, flags: MergeFlags) -> Self
    where
        SyncIndicatorPack<S>: Merge,
    {
        self.merge(other, flags);
        self
    }
}

impl IndicatorPack<Style> {
    pub fn resolve<F>(self, resolve_style: F) -> super::IndicatorPack
    where
        F: Fn(Style) -> super::Style,
    {
        super::IndicatorPack {
            sync: self.sync.resolve(resolve_style),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct SyncIndicatorPack<S = Style> {
    pub synced: Indicator<S>,
    pub failed: Indicator<S>,
}

impl Merge for SyncIndicatorPack<Style> {
    fn merge(&mut self, other: Self, flags: MergeFlags) {
        self.synced.merge(other.synced, flags);
        self.failed.merge(other.failed, flags);
    }
}

impl SyncIndicatorPack<Style> {
    pub fn resolve<F>(self, resolve_style: F) -> super::SyncIndicatorPack
    where
        F: Fn(Style) -> super::Style,
    {
        super::SyncIndicatorPack {
            synced: self.synced.resolve(&resolve_style),
            failed: self.failed.resolve(&resolve_style),
        }
    }
}

// SyncIndicatorPack Mergeable impls are in v1 module
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

impl Indicator<Style> {
    pub fn resolve<F>(self, resolve_style: F) -> super::Indicator
    where
        F: Fn(Style) -> super::Style,
    {
        super::Indicator {
            outer: self.outer.resolve(&resolve_style),
            inner: self.inner.resolve(&resolve_style),
            text: self.text,
        }
    }
}

impl<S: Clone> Indicator<S> {
    pub fn merge(&mut self, other: Self, flags: MergeFlags)
    where
        IndicatorStyle<S>: Merge,
    {
        self.outer.merge(other.outer, flags);
        self.inner.merge(other.inner, flags);
        if !other.text.is_empty() {
            self.text = other.text;
        }
    }

    pub fn merged(mut self, other: Self, flags: MergeFlags) -> Self
    where
        IndicatorStyle<S>: Merge,
    {
        self.merge(other, flags);
        self
    }
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

impl IndicatorStyle<Style> {
    pub fn resolve<F>(self, resolve_style: F) -> super::IndicatorStyle
    where
        F: Fn(Style) -> super::Style,
    {
        super::IndicatorStyle {
            prefix: self.prefix,
            suffix: self.suffix,
            style: resolve_style(self.style),
        }
    }
}

impl Merge for IndicatorStyle<Style> {
    fn merge(&mut self, other: Self, flags: MergeFlags) {
        if !other.prefix.is_empty() {
            self.prefix = other.prefix;
        }
        if !other.suffix.is_empty() {
            self.suffix = other.suffix;
        }
        self.style = std::mem::take(&mut self.style).merged(&other.style, flags);
    }
}

// ---

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

/// Convert v0::Style (`Vec<Mode>`) to v1::Style (ModeSetDiff)
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

/// Convert v0 indicator styles to v1
impl From<v0::IndicatorStyle> for IndicatorStyle<Style> {
    fn from(style: v0::IndicatorStyle) -> Self {
        IndicatorStyle {
            prefix: style.prefix,
            suffix: style.suffix,
            style: style.style.into(),
        }
    }
}

/// Convert v0 indicators to v1
impl From<v0::Indicator> for Indicator<Style> {
    fn from(indicator: v0::Indicator) -> Self {
        Indicator {
            outer: indicator.outer.into(),
            inner: indicator.inner.into(),
            text: indicator.text,
        }
    }
}

/// Convert v0 sync indicator packs to v1
impl From<v0::SyncIndicatorPack> for SyncIndicatorPack<Style> {
    fn from(pack: v0::SyncIndicatorPack) -> Self {
        SyncIndicatorPack {
            synced: pack.synced.into(),
            failed: pack.failed.into(),
        }
    }
}

/// Convert v0 indicator packs to v1
impl From<v0::IndicatorPack> for IndicatorPack<Style> {
    fn from(indicators: v0::IndicatorPack) -> Self {
        IndicatorPack {
            sync: indicators.sync.into(),
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
