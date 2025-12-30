// std imports
use std::collections::HashMap;

// third-party imports
use serde::Deserialize;

// relative imports
use super::{
    Color, Merge, MergeFlag, MergeFlags, ModeSet, ModeSetDiff, ResolvedStyle, Result, Role, StyleBase, StyleInventory,
    StylePack, StyleResolveError, v0,
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
        let base = base.into();
        Self { base, ..self }
    }

    pub fn modes(self, modes: impl Into<ModeSetDiff>) -> Self {
        let modes = modes.into();
        Self { modes, ..self }
    }

    pub fn background(self, background: Option<Color>) -> Self {
        Self { background, ..self }
    }

    pub fn foreground(self, foreground: Option<Color>) -> Self {
        Self { foreground, ..self }
    }

    pub fn reverse_merge(&mut self, other: Self, flags: MergeFlags) {
        *self = other.merged(&*self, flags);
    }

    /// Resolve role references in the `base` field and return a v1::Style with base=empty.
    /// This keeps ModeSetDiff intact (unlike `resolve()` which flattens to ModeSet).
    ///
    /// This is used for parent→inner merging where we need role properties resolved
    /// but mode diffs preserved for correct merge semantics.
    pub fn resolve_base(&self, inventory: &StyleInventory, flags: MergeFlags) -> Self {
        Self::resolve_base_with(&self.base, self, flags, |role| {
            inventory.get(role).cloned().unwrap_or_default()
        })
    }

    /// Resolve role references and return ResolvedStyle (flattens ModeSetDiff to ModeSet).
    pub fn resolve(&self, inventory: &StyleInventory, flags: MergeFlags) -> ResolvedStyle {
        // Reuse resolve_base logic, then convert to ResolvedStyle
        self.resolve_base(inventory, flags).as_resolved()
    }

    /// Generic base resolution that works with custom role resolvers.
    /// Returns v1::Style with base=empty but ModeSetDiff intact.
    pub fn resolve_base_with<F>(bases: &StyleBase, style: &Style, flags: MergeFlags, mut resolve_role: F) -> Style
    where
        F: FnMut(&Role) -> ResolvedStyle,
    {
        if bases.is_empty() {
            return style.clone();
        }

        // Resolve role references to get base properties
        let mut result = Style::default();
        for role in bases.iter() {
            // Convert ResolvedStyle back to v1::Style to preserve mode diff semantics
            let role_as_v1 = Style::from(resolve_role(role));
            result = result.merged(&role_as_v1, flags);
        }

        // Merge this style's explicit properties on top
        // We should NOT use ReplaceModes - the style's properties should be merged additively
        // with the base, not replace them. ReplaceModes is only for theme-level merging.
        result.merged(style, flags - MergeFlag::ReplaceModes)
    }

    /// Generic resolution with custom role resolver (for backward compatibility).
    /// Returns ResolvedStyle (flattens ModeSetDiff to ModeSet).
    pub fn resolve_with<F>(bases: &StyleBase, style: &Style, flags: MergeFlags, resolve_role: F) -> ResolvedStyle
    where
        F: FnMut(&Role) -> ResolvedStyle,
    {
        Self::resolve_base_with(bases, style, flags, resolve_role).as_resolved()
    }

    pub fn as_resolved(&self) -> ResolvedStyle {
        ResolvedStyle {
            modes: self.modes,
            foreground: self.foreground,
            background: self.background,
        }
    }

    fn merge_body(&mut self, other: &Self, flags: MergeFlags) {
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
    }
}

impl Default for &Style {
    fn default() -> Self {
        static DEFAULT: Style = Style::new();
        &DEFAULT
    }
}

impl Merge<Style> for Style {
    fn merge(&mut self, other: Style, flags: MergeFlags) {
        self.merge_body(&other, flags);
        if !other.base.is_empty() {
            self.base = other.base;
        }
    }
}

impl Merge<&Style> for Style {
    fn merge(&mut self, other: &Style, flags: MergeFlags) {
        self.merge_body(other, flags);
        if !other.base.is_empty() {
            self.base = other.base.clone();
        }
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

    pub fn resolve(&mut self, role: &Role) -> Result<ResolvedStyle, StyleResolveError> {
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

    fn resolve_style(&mut self, style: &Style, role: &Role) -> Result<ResolvedStyle, StyleResolveError> {
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
            return Err(StyleResolveError::RecursionLimitExceeded {
                role: *role,
                base,
                limit: RECURSION_LIMIT,
            });
        }

        // Role-to-role inheritance should use additive mode semantics (v1 style)
        // even when resolving v0 themes. ReplaceModes is only for v0 element merging.
        let role_flags = self.flags - MergeFlag::ReplaceModes;

        let mut result = ResolvedStyle::default();
        for base in base.iter() {
            result.merge(&self.resolve(base)?, role_flags);
        }

        Ok(result.merged(&style.as_resolved(), role_flags))
    }
}
