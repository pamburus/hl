//! Tells whether native notifications can be trusted for a path.

use std::path::Path;

// ---

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
use other as imp;

// ---

/// Whether native notifications can be trusted for changes of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reliability {
    /// The file system is known to be local, so every change to it is notified.
    KnownLocal,
    /// The file system is remote, unknown, or could not be told, so changes made elsewhere may not be notified.
    NotConfirmed,
}

/// Classifies the file system that serves `path`.
pub(crate) fn classify(path: &Path) -> Reliability {
    imp::classify(path)
}
