// std imports
use std::{cmp::Eq, collections::HashMap, hash::Hash};

// third-party imports
use derive_more::{Deref, DerefMut, IntoIterator};
use serde::Deserialize;

// relative imports
use super::{
    Element, Merge, MergeFlag, MergeFlags, MergeWithOptions, ResolvedStyle, Result, Role, Style, StyleInventory,
    StyleResolveError, StyleResolver,
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
    K: Deserialize<'de> + Eq + Hash,
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
    K: Eq + Hash,
{
    pub fn new(items: HashMap<K, S>) -> Self {
        Self(items)
    }
}

impl StylePack<Element, Style> {
    pub fn resolved(&self, inventory: &StyleInventory, flags: MergeFlags) -> StylePack<Element, ResolvedStyle> {
        // Per FR-041d: First resolve role refs (keeping ModeSetDiff), then parent→inner merge (with ModeSetDiff),
        // then convert to ResolvedStyle (flatten ModeSetDiff to ModeSet)
        self.clone()
            .resolve_base_refs(inventory, flags)
            .complete_hierarchy(flags)
            .convert_to_resolved()
    }

    /// Resolve role references in the `base` field but keep result as v1::Style with ModeSetDiff.
    /// This is step 4-5 of FR-041 but keeps ModeSetDiff intact for parent→inner merging.
    fn resolve_base_refs(self, inventory: &StyleInventory, flags: MergeFlags) -> Self {
        let items = self
            .iter()
            .map(|(&element, style)| (element, style.resolve_base(inventory, flags)))
            .collect();
        StylePack::new(items)
    }

    /// Completes the element hierarchy by applying parent→inner and boolean variant inheritance.
    /// This operates on v1::Style with base=empty but ModeSetDiff preserved.
    ///
    /// This is step 6 of FR-041 and must happen AFTER base resolution but BEFORE flattening ModeSetDiff.
    /// Operating on v1::Style (not ResolvedStyle) ensures:
    /// 1. Parent→inner merging preserves ModeSetDiff semantics (adds/removes)
    /// 2. Inner element's mode operations (e.g., "-faint") correctly override parent modes
    ///
    /// Per FR-041d, this ensures correct priority: inner's role properties > parent's explicit properties.
    fn complete_hierarchy(mut self, flags: MergeFlags) -> Self {
        // Step 1: Merge parent→inner where inner is explicitly defined (v1 only)
        // For v0 (ReplaceElements), inner elements replace parent completely
        if !flags.contains(MergeFlag::ReplaceElements) {
            // V1: Merge parent into each explicitly-defined inner element
            // Both have base=empty at this point, but ModeSetDiff is preserved
            for (element, style) in self.clone() {
                if let Some(outer) = element.outer() {
                    if let Some(outer) = self.0.get(&outer) {
                        // Merge parent and inner with ModeSetDiff semantics
                        self.0.insert(element, outer.clone().merged(&style, flags));
                    }
                }
            }
        }

        // Step 2: Add inherited inner elements that weren't explicitly defined
        // Use canonical pairs from Element::nested() for single source of truth (FR-015a)
        for &(outer, inner) in Element::nested() {
            if let Some(outer) = self.0.get(&outer).cloned() {
                self.0.entry(inner).or_insert(outer);
            }
        }

        // Step 3: Handle boolean variants inheriting from base boolean
        if let Some(base) = self.0.get(&Element::Boolean).cloned() {
            for variant in [Element::BooleanTrue, Element::BooleanFalse] {
                self.0
                    .entry(variant)
                    .and_modify(|style| {
                        // Merge base into variant: base properties fill in undefined properties
                        // Variant properties override base properties
                        *style = base.clone().merged(&*style, flags);
                    })
                    .or_insert_with(|| base.clone());
            }
        }

        self
    }

    /// Convert v1::Style to ResolvedStyle by flattening ModeSetDiff to ModeSet.
    /// This is the final step after all merging and inheritance is complete.
    fn convert_to_resolved(self) -> StylePack<Element, ResolvedStyle> {
        let items = self
            .iter()
            .map(|(&element, style)| (element, style.as_resolved()))
            .collect();
        StylePack::new(items)
    }
}

impl StylePack<Role, Style> {
    pub fn resolved(&self, flags: MergeFlags) -> Result<StyleInventory, StyleResolveError> {
        let mut resolver = StyleResolver::new(self, flags);
        let items: HashMap<Role, ResolvedStyle> = self
            .keys()
            .map(|k| resolver.resolve(k).map(|v| (*k, v)))
            .collect::<Result<_, _>>()?;
        Ok(StyleInventory::new(items))
    }
}

impl<S> MergeWithOptions for StylePack<Element, S>
where
    for<'a> S: MergeWithOptions<&'a S, Options = MergeFlags> + Default + Clone,
{
    type Options = MergeFlags;

    fn merge(&mut self, patch: Self, flags: MergeFlags) {
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

impl<S> MergeWithOptions<&StylePack<Element, S>> for StylePack<Element, S>
where
    for<'a> S: MergeWithOptions<&'a S, Options = MergeFlags> + Default + Clone,
    Self: MergeWithOptions<StylePack<Element, S>, Options = MergeFlags>,
{
    type Options = MergeFlags;

    fn merge(&mut self, other: &StylePack<Element, S>, options: MergeFlags) {
        <Self as MergeWithOptions<StylePack<Element, S>>>::merge(self, other.clone(), options);
    }
}

impl<S> Merge for StylePack<Role, S> {
    fn merge(&mut self, patch: Self) {
        self.0.extend(patch.0);
    }
}

#[cfg(test)]
pub(crate) mod tests;
