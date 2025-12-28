//! Theme configuration v1 format support.
//!
//! This module implements the v1 theme format, which adds semantic roles and
//! style inheritance on top of the legacy v0 format.
//!
//! # V1 Features
//!
//! - **Semantic Roles**: Define reusable style roles (primary, warning, error, etc.)
//! - **Style Inheritance**: Elements can inherit from roles using `style: [role-name]`
//! - **Multiple Inheritance**: Combine multiple roles: `style: [primary, strong]`
//! - **Mode Diffs**: Add/remove modes with `+`/`-` prefix: `modes: [+bold, -italic]`
//! - **Strict Validation**: Unknown fields/values cause errors (fail-fast)
//! - **$schema Support**: Optional `$schema` field for IDE validation
//!
//! # V1 Theme Structure
//!
//! ```yaml
//! version: "1.0"
//! tags: [dark]
//!
//! # Define semantic roles
//! styles:
//!   primary:
//!     foreground: "#00ff00"
//!     modes: [bold]
//!   warning:
//!     style: [primary]      # Inherit from primary
//!     foreground: "#ffaa00" # Override foreground
//!     modes: [+underline]   # Add underline to inherited modes
//!
//! # Map elements to roles
//! elements:
//!   level:
//!     style: [primary]
//!   timestamp:
//!     style: [warning]
//!
//! # Level-specific overrides
//! levels:
//!   error:
//!     level:
//!       foreground: "#ff0000"
//! ```
//!
//! # Recursion Limits and Circular Reference Protection
//!
//! Role-to-role inheritance chains are limited to a maximum depth of **64 levels**
//! to prevent stack overflow and infinite loops. This limit applies to both:
//!
//! - **Deep inheritance chains**: `role1 → role2 → role3 → ... → role64` (allowed)
//! - **Circular references**: `warning → error → warning` (rejected with error)
//!
//! When the limit is exceeded, theme loading fails with
//! [`ThemeLoadError::StyleRecursionLimitExceeded`], indicating which role
//! triggered the limit.
//!
//! ## Example Error
//!
//! ```yaml
//! # This will fail with recursion limit error:
//! styles:
//!   warning:
//!     style: [error]
//!   error:
//!     style: [warning]  # Circular reference!
//! ```
//!
//! Error message:
//! ```text
//! failed to resolve theme "my-theme": style recursion limit exceeded while resolving role warning
//! ```
//!
//! ## Specification References
//!
//! - **FR-046**: Maximum depth of 64 levels for role-to-role inheritance
//! - **FR-047**: Circular role references must be detected and cause an error
//!
//! # Implementation Details
//!
//! This module contains:
//! - V1-specific types ([`Role`], [`StyleBase`], [`Style`])
//! - Strict deserialization (fails on unknown keys/values)
//! - ALL merge logic for themes
//! - ALL resolve logic for themes (role → element resolution)
//! - Conversion from v0 to v1 format
//! - Recursion protection via [`StyleResolver`]

// std imports
use std::collections::HashMap;

// third-party imports
use derive_more::Deref;
use enumset::EnumSet;
use serde::{Deserialize, Serialize};

// local imports
use crate::level::{InfallibleLevel, Level};

// Import v0 module for conversion from v0 to v1
use super::v0;

// Re-export Element from v0 (unchanged in v1)
pub use super::v0::Element;

// Re-export common types from parent module
pub use super::{
    Color, MergeFlag, MergeFlags, Mode, ModeDiff, ModeDiffAction, ModeSet, ModeSetDiff, PlainColor, RGB, Result, Tag,
    ThemeLoadError, ThemeVersion,
};

// Import resolved types and traits from parent (output types)
use super::Merge;

// Import the resolved Style from parent (was ResolvedStyle)
use super::Style as ResolvedStyle;

// Type alias for resolved style inventory
pub type StyleInventory = super::StylePack<Role, ResolvedStyle>;

// Constants

/// Maximum depth for role-to-role style inheritance chains.
///
/// This limit prevents both excessively deep inheritance chains and circular
/// role references from causing stack overflow or infinite loops.
///
/// # Specification Requirements
///
/// - **FR-046**: V1 role-to-role inheritance via the `style` field MUST support
///   a maximum depth of 64 levels
/// - **FR-047**: V1 themes MUST detect circular role references (e.g.,
///   `warning: {style: "error"}` and `error: {style: "warning"}`) and exit
///   with error message
///
/// # Implementation
///
/// When resolving role styles, the [`StyleResolver`] tracks recursion depth.
/// If depth reaches this limit, resolution fails with
/// [`ThemeLoadError::StyleRecursionLimitExceeded`].
///
/// Circular references will be caught by this limit when the circular chain
/// is traversed 64 times. While this doesn't provide a full dependency chain
/// in the error message, it reliably prevents infinite loops and provides
/// a clear error indicating which role exceeded the limit.
///
/// # Examples
///
/// Valid deep chain (allowed up to 64 levels):
/// ```yaml
/// styles:
///   role1: { style: [role2] }
///   role2: { style: [role3] }
///   # ... up to 64 levels deep
/// ```
///
/// Circular reference (detected and rejected):
/// ```yaml
/// styles:
///   warning: { style: [error] }
///   error: { style: [warning] }  # Circular!
/// ```
const RECURSION_LIMIT: usize = 64;

// ---

/// Semantic style role for theme inheritance (v1 feature).
///
/// Roles allow defining reusable style definitions that elements can inherit from.
/// This enables DRY (Don't Repeat Yourself) theme definitions and consistent styling.
///
/// # Serialization
///
/// Roles are serialized in kebab-case to match user input format:
/// - `Primary` → `"primary"`
/// - `AccentSecondary` → `"accent-secondary"`
///
/// # Examples
///
/// In a theme file:
/// ```yaml
/// styles:
///   primary:
///     foreground: "#00ff00"
///   warning:
///     style: [primary]  # Inherit from primary role
///     background: "#331100"
///
/// elements:
///   level:
///     style: [primary]  # Use primary role
/// ```
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
/// Defines which roles a style should inherit from. Supports both single
/// and multiple role inheritance.
///
/// # Multiple Inheritance
///
/// When multiple roles are specified, they are merged **left to right**
/// (later roles override earlier ones):
///
/// ```yaml
/// styles:
///   combined:
///     style: [primary, strong]  # strong overrides primary
///     foreground: "#ff0000"     # this overrides both
/// ```
///
/// # Serialization Formats
///
/// - Single role: `style: "warning"` → `StyleBase(vec![Role::Warning])`
/// - Multiple roles: `style: ["primary", "strong"]` → `StyleBase(vec![Role::Primary, Role::Strong])`
/// - Empty (no inheritance): omitted or `null` → `StyleBase(vec![])`
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
            inventory.0.get(role).cloned().unwrap_or_default()
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
            modes: self.modes,
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
            modes: body.modes,
            foreground: body.foreground,
            background: body.background,
        }
    }
}

// ---

/// StylePack for v1 - strict deserialization, generic over key and style types
#[derive(Clone, Debug, Deref)]
pub struct StylePack<K, S = Style>(pub HashMap<K, S>);

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
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, key: &K) -> Option<&S> {
        self.0.get(key)
    }

    pub fn items(&self) -> &HashMap<K, S> {
        &self.0
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
        if flags.contains(MergeFlag::ReplaceGroups) {
            for (parent, child) in Element::pairs() {
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

impl Merge<&StylePack<Element, Style>> for StylePack<Element, Style> {
    fn merge(&mut self, other: &StylePack<Element, Style>, flags: MergeFlags) {
        Self::merge(self, other.clone(), flags);
    }
}

impl StylePack<Role, Style> {
    pub fn resolve(&self, flags: MergeFlags) -> Result<StyleInventory, ThemeLoadError> {
        let mut resolver = StyleResolver::new(self, flags);
        let items: HashMap<Role, ResolvedStyle> = self
            .0
            .keys()
            .map(|k| Ok((*k, resolver.resolve(k)?)))
            .collect::<Result<HashMap<Role, ResolvedStyle>, ThemeLoadError>>()?;
        Ok(super::StylePack(items))
    }
}

// ---

/// V1 theme definition (unresolved).
///
/// This is the deserialization target for v1 theme files. It contains unresolved
/// style definitions that may reference roles and use inheritance.
///
/// # Strict Validation
///
/// V1 themes use **strict deserialization** - unknown fields or enum variants
/// cause errors. This ensures themes are valid and helps catch typos early.
///
/// # Schema Support
///
/// V1 themes support an optional `$schema` field for IDE/validator integration:
///
/// ```yaml
/// $schema: "https://example.com/hl-theme-schema.json"
/// version: "1.0"
/// # ... rest of theme
/// ```
///
/// The `$schema` field is accepted but ignored during processing. It exists
/// purely to enable IDE features like autocomplete and validation.
///
/// # Examples
///
/// Basic v1 theme:
/// ```yaml
/// version: "1.0"
/// tags: [dark]
///
/// styles:
///   primary:
///     foreground: "#00ff00"
///     modes: [bold]
///
/// elements:
///   level:
///     style: [primary]
/// ```
///
/// # Resolution
///
/// Call [`Theme::resolve()`] to convert this unresolved theme to a fully
/// resolved [`super::Theme`] with all role references expanded.
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
        if flags.contains(MergeFlag::ReplaceGroups) {
            // Apply blocking rule: if child theme defines a parent element,
            // remove the corresponding -inner element from parent theme
            let parent_inner_pairs = [
                (Element::Level, Element::LevelInner),
                (Element::Logger, Element::LoggerInner),
                (Element::Caller, Element::CallerInner),
                (Element::InputNumber, Element::InputNumberInner),
                (Element::InputName, Element::InputNameInner),
            ];

            // Block base -inner elements if parent is defined in child theme
            for (parent, inner) in parent_inner_pairs {
                if other.elements.0.contains_key(&parent) {
                    self.elements.0.remove(&inner);
                }
            }

            // Block input-number/input-name and their inner elements if input is defined in child theme
            // This ensures v0 themes that define `input` get nested styling scope behavior
            if other.elements.0.contains_key(&Element::Input) {
                self.elements.0.remove(&Element::InputNumber);
                self.elements.0.remove(&Element::InputNumberInner);
                self.elements.0.remove(&Element::InputName);
                self.elements.0.remove(&Element::InputNameInner);
            }

            // Block entire level sections if child theme defines any element for that level
            for level in other.levels.keys() {
                self.levels.remove(level);
            }
        }

        // For both v0 and v1, elements defined in child theme replace elements from parent theme
        // Property-level merge happens later when merging elements with per-level styles
        self.elements.0.extend(other.elements.0);

        // For both v0 and v1, level-specific elements defined in child theme replace from parent
        for (level, pack) in other.levels {
            self.levels
                .entry(level)
                .and_modify(|existing| existing.0.extend(pack.0.clone()))
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
        let inventory = self.styles.resolve(flags)?;

        // Step 2: Resolve base element styles
        let elements = Self::resolve_element_pack(&self.elements, &inventory, flags)?;

        // Step 3: Resolve level-specific element styles
        let mut levels = HashMap::new();
        for (level, level_pack) in &self.levels {
            // Merge base elements with level-specific elements
            let merged_pack = self
                .elements
                .clone()
                .merged(level_pack, flags - MergeFlag::ReplaceGroups);
            let resolved_pack = Self::resolve_element_pack(&merged_pack, &inventory, flags)?;
            levels.insert(*level, resolved_pack);
        }

        // Step 4: Resolve indicators
        let indicators = Self::resolve_indicators(&self.indicators, &inventory, flags)?;

        Ok(super::Theme {
            tags: self.tags,
            version: self.version,
            elements,
            levels,
            indicators,
        })
    }

    fn resolve_element_pack(
        pack: &StylePack<Element, Style>,
        inventory: &StyleInventory,
        flags: MergeFlags,
    ) -> Result<super::StylePack<Element, ResolvedStyle>, ThemeLoadError> {
        let mut result = HashMap::new();

        let parent_inner_pairs = [
            (Element::Level, Element::LevelInner),
            (Element::Logger, Element::LoggerInner),
            (Element::Caller, Element::CallerInner),
            (Element::InputNumber, Element::InputNumberInner),
            (Element::InputName, Element::InputNameInner),
        ];

        // Process all elements, applying parent→inner inheritance where needed
        for (&element, style) in pack.items() {
            // Check if this element is an inner element that should inherit from its parent
            let mut parent_for_inner = None;
            for (parent, inner) in parent_inner_pairs.iter().copied() {
                if element == inner {
                    parent_for_inner = pack.items().get(&parent).cloned();
                    break;
                }
            }

            let resolved_style = match parent_for_inner {
                Some(parent_style) if !flags.contains(MergeFlag::ReplaceElements) => {
                    // V1: Resolve both parent and inner first, then merge based on resolved values
                    let resolved_inner = style.resolve(inventory, flags);
                    let resolved_parent = parent_style.resolve(inventory, flags);

                    // Parent fills in only properties that are None in the resolved inner
                    let mut merged = resolved_inner;
                    if merged.foreground.is_none() {
                        merged.foreground = resolved_parent.foreground;
                    }
                    if merged.background.is_none() {
                        merged.background = resolved_parent.background;
                    }
                    // For modes in v1, merge additively
                    merged.modes = resolved_parent.modes + merged.modes;

                    merged
                }
                _ => {
                    // V0 or no parent: just resolve the style
                    style.resolve(inventory, flags)
                }
            };

            result.insert(element, resolved_style);
        }

        // Add inherited inner elements that weren't explicitly defined
        for (parent, inner) in parent_inner_pairs.iter().copied() {
            if let Some(parent_style) = pack.items().get(&parent) {
                result
                    .entry(inner)
                    .or_insert_with(|| parent_style.resolve(inventory, flags));
            }
        }

        // Handle boolean variants inheriting from base boolean
        if let Some(base) = pack.items().get(&Element::Boolean) {
            for variant in [Element::BooleanTrue, Element::BooleanFalse] {
                let mut style = base.clone();
                if let Some(patch) = pack.items().get(&variant) {
                    style = style.merged(patch, flags)
                }
                result.insert(variant, style.resolve(inventory, flags));
            }
        }

        Ok(super::StylePack(result))
    }

    fn resolve_indicators(
        indicators: &IndicatorPack<Style>,
        inventory: &StyleInventory,
        flags: MergeFlags,
    ) -> Result<super::IndicatorPack<super::Style>, ThemeLoadError> {
        Ok(indicators.clone().resolve(|style| style.resolve(inventory, flags)))
    }

    pub fn merge_flags(&self) -> MergeFlags {
        match self.version {
            ThemeVersion { major: 0, .. } => {
                MergeFlag::ReplaceElements | MergeFlag::ReplaceGroups | MergeFlag::ReplaceModes
            }
            _ => MergeFlags::new(),
        }
    }

    /// Validate v1 theme version before deserialization
    ///
    /// V1 themes must be compatible with the current version
    /// This is called before deserialization to provide better error messages
    pub fn validate_version(version: &ThemeVersion) -> Result<(), super::ThemeLoadError> {
        // Default version (0.0/unspecified) is considered compatible with v1
        if *version == ThemeVersion::default() {
            return Ok(());
        }

        // Check if version is compatible with current supported version
        if !version.is_compatible_with(&ThemeVersion::CURRENT) {
            return Err(super::ThemeLoadError::UnsupportedVersion {
                requested: *version,
                supported: ThemeVersion::CURRENT,
            });
        }

        Ok(())
    }
}

// ---

/// StyleResolver - helper for resolving role-based styles with caching and recursion protection.
///
/// This resolver handles role-to-role inheritance chains while preventing:
/// - Stack overflow from excessively deep inheritance chains
/// - Infinite loops from circular role references
///
/// # Recursion Protection
///
/// The resolver enforces a maximum depth of 64 levels for style inheritance
/// chains. When this limit is exceeded, resolution fails with
/// [`ThemeLoadError::StyleRecursionLimitExceeded`].
///
/// This protects against both:
/// - Deep non-circular chains (e.g., role1 → role2 → ... → role65)
/// - Circular references (e.g., warning → error → warning)
///
/// # Caching
///
/// Resolved styles are cached to avoid redundant resolution of the same role.
/// This improves performance and ensures each role is resolved at most once.
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

        let style = self.inventory.0.get(role).unwrap_or_default();

        if self.depth >= RECURSION_LIMIT {
            return Err(ThemeLoadError::StyleRecursionLimitExceeded { role: *role });
        }

        self.depth += 1;
        let resolved = self.resolve_style(style, role)?;
        self.depth -= 1;

        self.cache.insert(*role, resolved.clone());

        Ok(resolved)
    }

    fn resolve_style(&mut self, style: &Style, role: &Role) -> Result<ResolvedStyle, ThemeLoadError> {
        // If no explicit base, default to inheriting from Default role (except for Default itself)
        let bases = if style.base.is_empty() {
            if *role != Role::Default {
                StyleBase::from(Role::Default)
            } else {
                StyleBase::default()
            }
        } else {
            style.base.clone()
        };

        let mut result = ResolvedStyle::default();
        for base_role in bases.0 {
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
    pub fn resolve<F>(self, resolve_style: F) -> super::IndicatorPack<super::Style>
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
    pub fn resolve<F>(self, resolve_style: F) -> super::SyncIndicatorPack<super::Style>
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
    pub fn resolve<F>(self, resolve_style: F) -> super::Indicator<super::Style>
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
    pub fn resolve<F>(self, resolve_style: F) -> super::IndicatorStyle<super::Style>
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
        let elements = theme.elements.0;
        let elements = elements.into_iter().map(|(e, style)| (e, style.into())).collect();

        // Convert v0 levels to v1 format
        let mut levels = HashMap::new();
        for (level, pack) in theme.levels {
            // Only convert valid levels - v1 is strict, invalid levels are dropped
            if let InfallibleLevel::Valid(level) = level {
                let pack = pack.0.into_iter().map(|(e, style)| (e, style.into())).collect();
                levels.insert(level, StylePack(pack));
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
            styles: StylePack(styles),
            elements: StylePack(elements),
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
