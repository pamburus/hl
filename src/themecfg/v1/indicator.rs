// third-party imports
use serde::Deserialize;

// relative imports
use super::{
    MergeFlags, MergeWithOptions, ResolvedIndicator, ResolvedIndicatorPack, ResolvedIndicatorStyle, ResolvedStyle,
    ResolvedSyncIndicatorPack, Style, v0,
};

// ---

/// Indicator types for v1 (generic over style type)
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct IndicatorPack<S = Style> {
    pub sync: SyncIndicatorPack<S>,
}

impl<S> MergeWithOptions for IndicatorPack<S>
where
    S: MergeWithOptions<Options = MergeFlags> + Default,
{
    type Options = MergeFlags;

    fn merge(&mut self, other: Self, options: MergeFlags) {
        self.sync.merge(other.sync, options);
    }
}

impl IndicatorPack<Style> {
    pub fn resolve<F>(self, resolve_style: F) -> ResolvedIndicatorPack
    where
        F: Fn(Style) -> ResolvedStyle,
    {
        ResolvedIndicatorPack {
            sync: self.sync.resolve(resolve_style),
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

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
#[serde(bound(deserialize = "S: Deserialize<'de> + Default"))]
pub struct SyncIndicatorPack<S = Style> {
    pub synced: Indicator<S>,
    pub failed: Indicator<S>,
}

impl<S> MergeWithOptions for SyncIndicatorPack<S>
where
    S: MergeWithOptions<Options = MergeFlags> + Default,
{
    type Options = MergeFlags;

    fn merge(&mut self, other: Self, options: Self::Options) {
        self.synced.merge(other.synced, options);
        self.failed.merge(other.failed, options);
    }
}

impl SyncIndicatorPack<Style> {
    pub fn resolve<F>(self, resolve_style: F) -> ResolvedSyncIndicatorPack
    where
        F: Fn(Style) -> ResolvedStyle,
    {
        ResolvedSyncIndicatorPack {
            synced: self.synced.resolve(&resolve_style),
            failed: self.failed.resolve(&resolve_style),
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

// ---

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
    pub fn resolve<F>(self, resolve_style: F) -> ResolvedIndicator
    where
        F: Fn(Style) -> ResolvedStyle,
    {
        ResolvedIndicator {
            outer: self.outer.resolve(&resolve_style),
            inner: self.inner.resolve(&resolve_style),
            text: self.text,
        }
    }
}

impl<S> MergeWithOptions for Indicator<S>
where
    S: MergeWithOptions<Options = MergeFlags> + Default,
{
    type Options = MergeFlags;

    fn merge(&mut self, other: Self, flags: MergeFlags) {
        self.outer.merge(other.outer, flags);
        self.inner.merge(other.inner, flags);
        if !other.text.is_empty() {
            self.text = other.text;
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

// ---

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
    pub fn resolve<F>(self, resolve_style: F) -> ResolvedIndicatorStyle
    where
        F: Fn(Style) -> ResolvedStyle,
    {
        ResolvedIndicatorStyle {
            prefix: self.prefix,
            suffix: self.suffix,
            style: resolve_style(self.style),
        }
    }
}

impl<S> MergeWithOptions for IndicatorStyle<S>
where
    S: MergeWithOptions<Options = MergeFlags> + Default,
{
    type Options = MergeFlags;

    fn merge(&mut self, other: Self, options: Self::Options) {
        if !other.prefix.is_empty() {
            self.prefix = other.prefix;
        }
        if !other.suffix.is_empty() {
            self.suffix = other.suffix;
        }
        self.style = std::mem::take(&mut self.style).merged(other.style, options);
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

#[cfg(test)]
pub(crate) mod tests;
