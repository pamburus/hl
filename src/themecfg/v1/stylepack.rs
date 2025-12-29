// std imports
use std::collections::HashMap;

// third-party imports
use derive_more::{Deref, DerefMut, IntoIterator};
use serde::Deserialize;

// relative imports
use super::{
    Element, Merge, MergeFlag, MergeFlags, ResolvedStyle, ResolvedStylePack, Result, Role, Style, StyleInventory,
    StyleResolver, ThemeLoadError,
};

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
    pub fn resolve_styles(&self, inventory: &StyleInventory, flags: MergeFlags) -> ResolvedStylePack {
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
