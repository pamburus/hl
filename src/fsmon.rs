// std imports
use std::path::PathBuf;

// local imports
use crate::error::Result;

// ---

/// A change of a watched file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The file was created, written to, renamed or changed otherwise, so it can have new content
    /// or be another file than before.
    Modified,
    /// The file was removed.
    Removed,
}

#[cfg(not(target_os = "macos"))]
impl Change {
    /// Returns the change reported by a notify event of `kind`, or `None` for an access event.
    ///
    /// Access events are dropped, including the one reporting that a file opened for writing was closed:
    /// the writes themselves are reported as modifications.
    /// An event of unknown kind is taken as a modification, so the file is checked.
    fn from_kind(kind: &notify::EventKind) -> Option<Self> {
        use notify::EventKind;

        match kind {
            EventKind::Access(_) => None,
            EventKind::Remove(_) => Some(Self::Removed),
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Any | EventKind::Other => Some(Self::Modified),
        }
    }
}

/// Returns whether one of the `changed` paths is one of `paths`, sorted.
fn concerns(changed: &[PathBuf], paths: &[PathBuf]) -> bool {
    changed.iter().any(|path| paths.binary_search(path).is_ok())
}

// ---

/// Watches `paths` and calls `handle` for each change of one of the files, until it fails.
pub fn run<H>(mut paths: Vec<PathBuf>, mut handle: H) -> Result<()>
where
    H: FnMut(Change) -> Result<()>,
{
    if paths.is_empty() {
        return Ok(());
    }

    paths.retain(|path| path.metadata().is_ok_and(|metadata| metadata.file_type().is_file()));

    for i in 0..paths.len() {
        if let Ok(canonical_path) = paths[i].canonicalize() {
            match paths[i].symlink_metadata() {
                Ok(metadata) if metadata.file_type().is_symlink() => paths.push(canonical_path),
                _ => paths[i] = canonical_path,
            }
        }
    }

    paths.sort_unstable();
    paths.dedup();

    let mut watch = paths
        .iter()
        .map(|path| {
            let mut path = path.clone();
            path.pop();
            path
        })
        .collect::<Vec<PathBuf>>();
    watch.extend_from_slice(&paths);
    watch.sort_unstable();
    watch.dedup();

    imp::run(watch, |change, changed| {
        if concerns(changed, &paths) {
            handle(change)
        } else {
            Ok(())
        }
    })
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(not(any(target_os = "macos", windows)))]
mod imp {
    use std::sync::mpsc::{self};
    use std::time::Duration;

    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    use super::*;
    use crate::error::Error;

    const FALLBACK_POLLING_INTERVAL: Duration = Duration::from_secs(1);

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H) -> Result<()>
    where
        H: FnMut(Change, &[PathBuf]) -> Result<()>,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

        for path in &paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    if let Some(change) = Change::from_kind(&event.kind) {
                        handle(change, &event.paths)?;
                    }
                }
                Ok(Err(err)) => return Err(err.into()),
                Err(err) => return Err(Error::RecvTimeoutError { source: err.into() }),
            };
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::{collections::HashSet, time::Duration};

    use kqueue::{EventData, EventFilter, FilterFlag, Ident, Vnode, Watcher};

    use super::*;

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H) -> Result<()>
    where
        H: FnMut(Change, &[PathBuf]) -> Result<()>,
    {
        let mut watcher = Watcher::new()?;
        let mut added = HashSet::<&PathBuf>::new();
        let mut synced = true;

        // NOTE_ATTRIB is required to detect in-place truncation (O_TRUNC) on APFS/macOS.
        // APFS fires NOTE_ATTRIB for file-size metadata changes (truncation) rather than
        // NOTE_WRITE, so without this flag the follower misses the transient size=0 state
        // and cannot seek to offset 0 before the subsequent write fills the file again.
        let flags = FilterFlag::NOTE_FFNOP
            | FilterFlag::NOTE_DELETE
            | FilterFlag::NOTE_WRITE
            | FilterFlag::NOTE_RENAME
            | FilterFlag::NOTE_EXTEND
            | FilterFlag::NOTE_ATTRIB;

        loop {
            for path in &paths {
                if watcher.add_filename(path, EventFilter::EVFILT_VNODE, flags).is_ok() {
                    added.insert(path);
                    if !synced {
                        handle(Change::Modified, std::slice::from_ref(path))?;
                    }
                }
            }

            synced = true;
            watcher.watch()?;

            while synced {
                let event = if let Some(event) = watcher.poll(Some(Duration::from_secs(1))) {
                    event
                } else {
                    continue;
                };

                match event {
                    kqueue::Event {
                        data: EventData::Vnode(data),
                        ident: Ident::Filename(_, path),
                    } => {
                        let path = PathBuf::from(path);
                        let change = match data {
                            Vnode::Delete | Vnode::Revoke => {
                                if added.contains(&path) {
                                    watcher.remove_filename(&path, EventFilter::EVFILT_VNODE)?;
                                    added.remove(&path);
                                    synced = false;
                                }
                                Change::Removed
                            }
                            Vnode::Write => {
                                if added.len() < paths.len() && path.is_dir() {
                                    synced = false;
                                }
                                Change::Modified
                            }
                            Vnode::Rename => {
                                if added.contains(&path) {
                                    watcher.remove_filename(&path, EventFilter::EVFILT_VNODE)?;
                                    added.remove(&path);
                                    synced = false;
                                }
                                Change::Modified
                            }
                            // Extended, truncated, linked or attributes changed.
                            _ => Change::Modified,
                        };
                        handle(change, std::slice::from_ref(&path))?;
                    }
                    _ => unreachable!(),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests;
