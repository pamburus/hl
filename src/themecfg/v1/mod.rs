//! Theme configuration v1 format support.
//!
//! Implements strict, semantic theme loading with role-based inheritance.
//! Supports `$schema`, mode diffs, and deep inheritance chains (up to 64 levels).

// relative imports
use super::{
    Color, Element, Indicator as ResolvedIndicator, IndicatorPack as ResolvedIndicatorPack,
    IndicatorStyle as ResolvedIndicatorStyle, Merge, MergeFlag, MergeFlags, ModeSet, ModeSetDiff, Result,
    Style as ResolvedStyle, StyleInventory, StylePack as ResolvedStylePack,
    SyncIndicatorPack as ResolvedSyncIndicatorPack, Tag, Theme as ResolvedTheme, ThemeLoadError, Version, v0,
};

// sub-modules
mod indicator;
mod role;
mod style;
mod stylebase;
mod stylepack;
mod theme;

// Re-export commonly used types
pub use {indicator::*, role::*, style::*, stylebase::*, stylepack::*, theme::*};
