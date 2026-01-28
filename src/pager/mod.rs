//! Pager configuration and selection system.
//!
//! This module provides support for configuring and selecting pagers for output pagination,
//! including profile-based configuration, role-specific arguments (view vs. follow mode),
//! and priority-based fallback selection.

mod config;
mod selection;

#[cfg(test)]
mod tests;

pub use config::{PagerConfig, PagerProfile, PagerRole, PagerRoleConfig};
pub use selection::{
    EnvProvider, ExeChecker, PagerSelector, PagerSpec, SelectedPager, SystemEnv, SystemExeChecker, is_available,
};
