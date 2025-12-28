// std imports
use std::sync::Arc;

// third-party imports
use derive_more::Deref;

// relative imports
use super::{Error, MergeFlags, Result, Theme, ThemeInfo, ThemeOrigin, ThemeSource, v1};

/// An unresolved theme with metadata, before style resolution.
///
/// Wraps a [`v1::Theme`] and includes metadata (name, source) for error reporting.
/// Can be modified before calling `.resolve()` to create a usable [`Theme`].
#[derive(Debug, Clone, Deref)]
pub struct RawTheme {
    /// Theme metadata (name, source, origin).
    pub info: Arc<ThemeInfo>,
    /// The unresolved theme data.
    #[deref]
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

    /// Merge this theme with another theme.
    ///
    /// The `other` theme's values override this theme's values where they conflict.
    pub fn merged(self, other: Self) -> Self {
        Self {
            info: other.info,
            inner: self.inner.merged(other.inner),
        }
    }

    /// Get the merge flags from this theme.
    pub fn merge_flags(&self) -> MergeFlags {
        self.inner.merge_flags()
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

impl Default for RawTheme {
    fn default() -> Self {
        Self {
            info: ThemeInfo::new("(empty)", ThemeSource::Embedded, ThemeOrigin::Stock).into(),
            inner: v1::Theme::default(),
        }
    }
}

impl std::ops::DerefMut for RawTheme {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
