use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

// ---

pub type Inner = (u64, u64);

pub fn from_file(file: &File) -> std::io::Result<Inner> {
    let meta = file.metadata()?;
    Ok((meta.dev(), meta.ino()))
}

pub fn from_path(path: &Path) -> std::io::Result<Inner> {
    let meta = std::fs::metadata(path)?;
    Ok((meta.dev(), meta.ino()))
}
