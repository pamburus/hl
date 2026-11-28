// std imports
use std::path::PathBuf;

// local imports
use crate::error::Result;

// ---

pub type Event = notify::Event;
pub type EventKind = notify::EventKind;

// ---

pub fn run<H>(mut paths: Vec<PathBuf>, handle: H) -> Result<()>
where
    H: FnMut(Event) -> Result<()>,
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

    imp::run(watch, Filter { paths }, handle)
}

// ---

/// Tells which events are worth handling.
struct Filter {
    /// The files watched, sorted.
    paths: Vec<PathBuf>,
}

impl Filter {
    /// Returns whether `event` is about one of the files watched and can change what is read from it.
    ///
    /// A watched directory brings the events of all files in it, and every read of a file brings an access event.
    fn accepts(&self, event: &Event) -> bool {
        !event.kind.is_access() && event.paths.iter().any(|path| self.paths.binary_search(path).is_ok())
    }
}

// ---

#[cfg(not(target_os = "macos"))]
mod queue;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(not(any(target_os = "macos", windows)))]
mod imp {
    use std::{sync::Arc, time::Duration};

    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    use super::{queue::Queue, *};

    const FALLBACK_POLLING_INTERVAL: Duration = Duration::from_secs(1);

    pub fn run<H>(paths: Vec<PathBuf>, filter: Filter, mut handle: H) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        let queue = Arc::new(Queue::new(filter));
        let push = {
            let queue = queue.clone();
            move |event| queue.push(event)
        };
        let config = Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL);
        let mut watcher = RecommendedWatcher::new(push, config)?;

        for path in &paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        loop {
            handle(queue.pop()?)?;
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::{collections::HashSet, time::Duration};

    use kqueue::{EventData, EventFilter, FilterFlag, Ident, Vnode, Watcher};
    use notify::event::{CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode};

    use super::*;

    pub fn run<H>(paths: Vec<PathBuf>, filter: Filter, mut handle: H) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
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
                        let event = Event::new(EventKind::Create(CreateKind::Any)).add_path(path.clone());
                        if filter.accepts(&event) {
                            handle(event)?;
                        }
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
                        let event = match data {
                            Vnode::Delete | Vnode::Revoke => {
                                if added.contains(&path) {
                                    watcher.remove_filename(&path, EventFilter::EVFILT_VNODE)?;
                                    added.remove(&path);
                                    synced = false;
                                }
                                Event::new(EventKind::Remove(RemoveKind::Any)).add_path(path)
                            }
                            Vnode::Write => {
                                if added.len() < paths.len() && path.is_dir() {
                                    synced = false;
                                }
                                Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Any))).add_path(path)
                            }
                            Vnode::Extend | Vnode::Truncate => {
                                Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Size))).add_path(path)
                            }
                            Vnode::Rename => {
                                if added.contains(&path) {
                                    watcher.remove_filename(&path, EventFilter::EVFILT_VNODE)?;
                                    added.remove(&path);
                                    synced = false;
                                }
                                Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Any))).add_path(path)
                            }
                            Vnode::Link => Event::new(EventKind::Create(CreateKind::Any)).add_path(path),
                            Vnode::Attrib => {
                                Event::new(EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any))).add_path(path)
                            }

                            #[allow(unreachable_patterns)]
                            _ => Event::new(EventKind::Other),
                        };
                        if filter.accepts(&event) {
                            handle(event)?;
                        }
                    }
                    _ => unreachable!(),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests;
