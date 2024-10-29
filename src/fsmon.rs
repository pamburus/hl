// std imports
use std::path::PathBuf;

// local imports
use crate::error::Result;

// ---

pub type Event = notify::Event;
pub type EventKind = notify::EventKind;

// ---

pub fn run<H>(mut paths: Vec<PathBuf>, mut handle: H) -> Result<()>
where
    H: FnMut(Event) -> Result<()>,
{
    if paths.is_empty() {
        return Ok(());
    }

    paths.retain(|path| path.metadata().map_or(false, |metadata| metadata.file_type().is_file()));

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

    imp::run(watch, |event| {
        if event.paths.iter().any(|path| paths.binary_search(&path).is_ok()) {
            handle(event)
        } else {
            Ok(())
        }
    })
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::sync::mpsc::{self};
    use std::time::Duration;

    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    use super::*;
    use crate::error::Error;

    const FALLBACK_POLLING_INTERVAL: Duration = Duration::from_secs(1);

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

        for path in &paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        loop {
            match rx.recv() {
                Ok(Ok(event)) => handle(event)?,
                Ok(Err(err)) => return Err(err.into()),
                Err(err) => return Err(Error::RecvTimeoutError { source: err.into() }),
            };
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::collections::HashSet;

    use kqueue::{EventData, EventFilter, FilterFlag, Ident, Vnode, Watcher};
    use notify::event::{CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode};

    use super::*;

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        let mut watcher = Watcher::new()?;
        let mut added = HashSet::<&PathBuf>::new();
        let mut synced = true;

        let flags = FilterFlag::NOTE_FFNOP
            | FilterFlag::NOTE_DELETE
            | FilterFlag::NOTE_WRITE
            | FilterFlag::NOTE_RENAME
            | FilterFlag::NOTE_EXTEND;

        loop {
            for path in &paths {
                if watcher.add_filename(path, EventFilter::EVFILT_VNODE, flags).is_ok() {
                    added.insert(path);
                    if !synced {
                        handle(Event::new(EventKind::Create(CreateKind::Any)).add_path(path.clone()))?;
                    }
                }
            }

            synced = true;
            watcher.watch()?;

            while synced {
                let event = if let Some(event) = watcher.poll(None) {
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
                        handle(event)?;
                    }
                    _ => unreachable!(),
                };
            }
        }
    }
}
