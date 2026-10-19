// std imports
use std::{fs::Metadata, os::unix::fs::MetadataExt};

// ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileId {
    dev: u64,
    ino: u64,
}

#[inline]
pub fn file_id(meta: &Metadata) -> Option<FileId> {
    Some(FileId {
        dev: meta.dev(),
        ino: meta.ino(),
    })
}

#[cfg(test)]
mod tests;
