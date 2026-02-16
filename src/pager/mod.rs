//! Pager configuration and selection system.
//!
//! This module provides support for configuring and selecting pagers for output pagination,
//! including profile-based configuration, role-specific arguments (view vs. follow mode),
//! and priority-based fallback selection.

mod config;
mod selection;

#[cfg(test)]
mod tests;

use pager::PagerProcess;

pub use config::{PagerConfig, PagerProfile, PagerRole, PagerRoleConfig};
pub use selection::{
    EnvProvider, Error, ExeChecker, PagerOverride, PagerSelector, SelectedPager, SystemEnv, SystemExeChecker,
    is_available,
};

/// Monitors a pager process and invokes a callback with the outcome when it exits.
///
/// On drop, waits for the pager process to finish, recovers terminal state, and
/// calls the provided callback with `Ok(())` on success or `Err(Error)` on failure.
pub struct PagerWatcher<F: FnOnce(Result<(), Error>)> {
    process: PagerProcess,
    on_exit: Option<F>,
}

impl<F> PagerWatcher<F>
where
    F: FnOnce(Result<(), Error>),
{
    /// Creates a new pager watcher that will call `on_exit` when the pager process exits.
    pub fn new(process: PagerProcess, on_exit: F) -> Self {
        Self {
            process,
            on_exit: Some(on_exit),
        }
    }
}

impl<F> Drop for PagerWatcher<F>
where
    F: FnOnce(Result<(), Error>),
{
    fn drop(&mut self) {
        let result = match self.process.wait() {
            Ok(result) => {
                log::debug!("pager process exited with status: {:?}", result.status);
                if result.is_success() {
                    Ok(())
                } else {
                    let exit_code = result.exit_code().unwrap_or(141);
                    Err(Error::PagerFailed {
                        command: result.command,
                        exit_code,
                        stderr: result.stderr,
                    })
                }
            }
            Err(err) => {
                log::debug!("failed to wait for pager: {err}");
                Err(Error::WaitFailed { source: err })
            }
        };

        if let Some(on_exit) = self.on_exit.take() {
            on_exit(result);
        }
    }
}
