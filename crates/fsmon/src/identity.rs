//! Tells whether two opens are of the same file.

use std::{fs::File, io, path::Path};

// ---

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as imp;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

// ---

/// The identity of a file, which stays the same while the file is renamed and differs for a file put in its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileId(imp::Id);

impl FileId {
    /// Returns the identity of an open file.
    pub fn of_file(file: &File) -> io::Result<Self> {
        imp::of_file(file).map(Self)
    }

    /// Returns the identity of the file at `path`, following symbolic links.
    pub fn of_path(path: &Path) -> io::Result<Self> {
        imp::of_path(path).map(Self)
    }

    /// Returns an identity made up for a test, which is the same for the same `n`.
    #[cfg(test)]
    pub fn fake(n: u8) -> Self {
        Self((0, n.into()))
    }
}
