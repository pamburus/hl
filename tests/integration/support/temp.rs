use std::{
    fs,
    path::{Path, PathBuf},
};

/// An empty temporary directory that is removed when dropped, also when a test fails.
pub struct TempDir(PathBuf);

impl TempDir {
    /// Creates an empty directory, with `name` telling it from the directories of other tests in the same process.
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("hl-test-{}-{}", std::process::id(), name));
        // A directory left by an earlier process with the same ID.
        fs::remove_dir_all(&dir).ok();
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }

    /// Returns the path of `name` in the directory.
    pub fn join(&self, name: impl AsRef<Path>) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}
