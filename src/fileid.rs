// std imports
use std::fs::Metadata;

// ---

/// Identity of a file that survives renames, so a file replaced by another one
/// under the same path, as during log rotation, can be told apart from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileId(imp::FileId);

// ---

pub trait MetadataExt {
    /// Returns the identity of the file, if the platform provides it as part of
    /// the file metadata.
    fn file_id(&self) -> Option<FileId>;
}

impl MetadataExt for Metadata {
    #[inline]
    fn file_id(&self) -> Option<FileId> {
        imp::file_id(self).map(FileId)
    }
}

// ---

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as imp;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(test)]
mod tests;
