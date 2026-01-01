// std imports
use std::sync::{Arc, LazyLock};

// third-party imports
use derive_more::{Deref, DerefMut};

use crate::themecfg::Merge;

// relative imports
use super::{Assets, Error, MergeFlags, MergeOptions, Result, Tag, Theme, ThemeInfo, ThemeOrigin, ThemeSource, v1};

static BASE: LazyLock<RawTheme> = LazyLock::new(|| Theme::load_embedded::<Assets>("@base").unwrap());

/// An unresolved theme with metadata, before style resolution.
///
/// Wraps a [`v1::Theme`] and includes metadata (name, source) for error reporting.
/// Can be modified before calling `.resolve()` to create a usable [`Theme`].
#[derive(Debug, Clone, Deref, DerefMut)]
pub struct RawTheme {
    /// Theme metadata (name, source, origin).
    pub info: Arc<ThemeInfo>,
    /// The unresolved theme data.
    #[deref]
    #[deref_mut]
    inner: v1::Theme,
}

impl RawTheme {
    /// Create a new `RawTheme` with metadata.
    pub fn new(info: impl Into<Arc<ThemeInfo>>, inner: v1::Theme) -> Self {
        Self {
            info: info.into(),
            inner,
        }
    }

    /// Access the base theme.
    ////
    /// The base theme provides default styles for all elements.
    pub fn base() -> &'static RawTheme {
        &BASE
    }

    /// Resolve the theme to a fully resolved [`Theme`].
    ///
    /// Resolves all role-based styles to concrete element styles.
    /// Errors will include the theme name and source from metadata.
    pub fn resolve(self) -> Result<Theme> {
        self.inner.resolve().map_err(|source| Error::FailedToResolveTheme {
            info: self.info.clone(),
            source,
        })
    }

    /// Access the inner v1::Theme for advanced use cases.
    pub fn inner(&self) -> &v1::Theme {
        &self.inner
    }

    /// Access the inner v1::Theme mutably for advanced use cases.
    pub fn inner_mut(&mut self) -> &mut v1::Theme {
        &mut self.inner
    }

    /// Consume self and return the inner v1::Theme.
    pub fn into_inner(self) -> v1::Theme {
        self.inner
    }
}

impl Merge for RawTheme {
    fn merge(&mut self, other: Self) {
        let base_name = self.info.name.clone();
        let patch_name = other.info.name.clone();
        let patch_version = other.inner.version;

        log::debug!(
            "merging theme '{}' (v{}) into '{}' (v{})",
            patch_name,
            patch_version,
            base_name,
            self.inner.version
        );

        if !other.inner.tags.contains(Tag::Overlay) {
            log::trace!("theme '{}' is not tagged as overlay, updating metadata", patch_name);
            self.info = other.info;
        } else {
            log::trace!("theme '{}' is tagged as overlay, preserving base metadata", patch_name);
        }

        self.inner.merge(other.inner);
        log::trace!("merge complete: '{}' <- '{}'", base_name, patch_name);
    }
}

impl Default for RawTheme {
    fn default() -> Self {
        Self {
            info: ThemeInfo::new("(empty)", ThemeSource::Embedded, ThemeOrigin::Stock).into(),
            inner: v1::Theme::default(),
        }
    }
}

impl MergeOptions for RawTheme {
    type Output = MergeFlags;

    fn merge_options(&self) -> Self::Output {
        self.inner.merge_options()
    }
}

#[cfg(test)]
pub(crate) mod tests;
