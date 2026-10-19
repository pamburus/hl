// std imports
use std::fs::Metadata;

// ---

/// Standard file metadata carries no stable file identity on Windows, so there
/// are no values of this type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileId {}

#[inline]
pub fn file_id(_: &Metadata) -> Option<FileId> {
    None
}
