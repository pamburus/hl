//! Waits for changes of a followed path.

use std::{
    ffi::OsString,
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use crate::stop::Stop;

// ---

#[cfg(unix)]
pub(crate) mod unix;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
use other as imp;

// ---

/// Why a wait ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wake {
    /// Something may have changed.
    Ready,
    /// The timeout passed.
    Timeout,
    /// The follow was stopped.
    Stopped,
}

/// Native notifications of changes to a path.
pub(crate) struct Watch(imp::Watch);

impl Watch {
    /// Watches for changes to the file at `path`, its replacement, and its removal.
    ///
    /// The file is watched through the directories that hold the path and the file the path resolves to,
    /// so `path` may be missing. The open `file`, if given, may also be watched on its own.
    pub fn new(path: &Path, file: Option<&File>, stop: &Arc<Stop>) -> io::Result<Self> {
        imp::Watch::new(&entries(path), file, stop.clone()).map(Self)
    }

    /// Waits until something may have changed, the timeout passes, or the follow is stopped.
    pub fn wait(&mut self, timeout: Option<Duration>) -> io::Result<Wake> {
        self.0.wait(timeout)
    }
}

/// A directory entry to watch, as the directory and the name within it.
pub(crate) type Entry = (PathBuf, OsString);

/// Returns the directory that holds `path`, which is the current one for a bare name.
pub(crate) fn dir_of(path: &Path) -> &Path {
    match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    }
}

/// The most links followed to find the target of a link whose target is missing, as many as Linux follows.
const MAX_LINKS: usize = 40;

/// Returns the entries of `path`, of the links it leads through and of the file it resolves to.
///
/// A link it leads through is watched too, so that pointing it to another file is seen, and so is a target that is
/// missing, so that it is seen once it appears.
fn entries(path: &Path) -> Vec<Entry> {
    let mut entries = Vec::with_capacity(2);
    let mut add = |path: &Path| {
        if let Some(name) = path.file_name() {
            let entry = (dir_of(path).to_owned(), name.to_owned());
            if !entries.contains(&entry) {
                entries.push(entry);
            }
        }
    };
    add(path);
    if let Ok(target) = std::fs::canonicalize(path) {
        add(&target);
    }
    let mut link = path.to_owned();
    for _ in 0..MAX_LINKS {
        let Ok(target) = std::fs::read_link(&link) else {
            break;
        };
        // A relative target is relative to the directory of the link, and an absolute one takes the place of it all.
        link.set_file_name(target);
        add(&link);
    }
    entries
}

// ---

#[cfg(test)]
mod tests;
