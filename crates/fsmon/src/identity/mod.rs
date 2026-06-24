use std::fs::File;
use std::path::Path;

// ---

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::open_shared;

// ---

/// Stable identifier for an open file. Two opens of the same file compare
/// equal; a same-path replacement compares unequal (FR-005, FR-019).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileId(Inner);

#[cfg(unix)]
type Inner = unix::Inner;
#[cfg(windows)]
type Inner = windows::Inner;
#[cfg(not(any(unix, windows)))]
type Inner = ();

impl FileId {
    /// Obtain the `FileId` for an already-open file handle.
    #[allow(unused_variables)]
    pub fn from_file(file: &File) -> std::io::Result<Self> {
        #[cfg(unix)]
        return unix::from_file(file).map(FileId);
        #[cfg(windows)]
        return windows::from_file(file).map(FileId);
        #[cfg(not(any(unix, windows)))]
        return Ok(FileId(()));
    }

    /// Obtain the `FileId` for a path without opening it.
    #[allow(unused_variables)]
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        #[cfg(unix)]
        return unix::from_path(path).map(FileId);
        #[cfg(windows)]
        return windows::from_path(path).map(FileId);
        #[cfg(not(any(unix, windows)))]
        return Ok(FileId(()));
    }
}
