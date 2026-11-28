use std::path::{Path, PathBuf};

/// The path of `file` in the repository.
pub fn repo_file(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(file)
}
