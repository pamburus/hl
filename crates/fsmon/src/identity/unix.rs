use std::{fs::File, io, os::unix::fs::MetadataExt, path::Path};

// ---

/// The device and the inode.
pub type Id = (u64, u64);

pub fn of_file(file: &File) -> io::Result<Id> {
    let meta = file.metadata()?;
    Ok((meta.dev(), meta.ino()))
}

pub fn of_path(path: &Path) -> io::Result<Id> {
    let meta = std::fs::metadata(path)?;
    Ok((meta.dev(), meta.ino()))
}
