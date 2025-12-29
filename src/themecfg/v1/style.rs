// std imports
use std::collections::HashMap;

// third-party imports
use serde::Deserialize;

// relative imports
use super::{
    Color, Merge, MergeFlag, MergeFlags, ModeSet, ModeSetDiff, ResolvedStyle, Result, Role, StyleBase, StyleInventory,
    StylePack, ThemeLoadError, v0,
};

// ---

// Constants

/// Maximum depth for role-to-role style inheritance chains.
///
/// Limits recursion depth to 64 (FR-046) to prevent infinite loops and stack overflow.
/// Circular references will trigger this limit (FR-047).
const RECURSION_LIMIT: usize = 64;

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
            base: StyleBase::new(),
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
    pub fn new(inventory: &'a StylePack<Role, Style>, flags: MergeFlags) -> Self {
        Self {
            inventory,
            flags,
            cache: HashMap::new(),
            depth: 0,
        }
    }

    pub fn resolve(&mut self, role: &Role) -> Result<ResolvedStyle, ThemeLoadError> {
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
        for base in base.iter() {
            result = result.merged(&self.resolve(&base)?, self.flags);
        }

        Ok(result.merged(&style.as_resolved(), self.flags))
    }
}
