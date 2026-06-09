use std::path::Path;

// ---

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

// ---

/// Per-path reliability classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reliability {
    /// Affirmatively known-local filesystem. Native notifications are reliable.
    KnownLocal,
    /// Could not be confirmed as local: known-remote, FUSE, unknown, or
    /// `statfs` error. Conservative default — polling is always correct.
    NotConfirmed,
}

/// Classify the reliability of the filesystem serving `path`.
///
/// Errors and unrecognized types both yield `NotConfirmed` (conservative).
pub fn classify(path: &Path) -> Reliability {
    #[cfg(target_os = "linux")]
    return linux::classify(path);
    #[cfg(target_os = "macos")]
    return macos::classify(path);
    #[cfg(windows)]
    return windows::classify(path);
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    return Reliability::NotConfirmed;
}
