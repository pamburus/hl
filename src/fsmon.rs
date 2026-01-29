// std imports
use std::path::PathBuf;
use std::time::Duration;

// local imports
use crate::error::Result;

// ---

pub type Event = notify::Event;
pub type EventKind = notify::EventKind;

/// Result of handling an event - indicates whether to continue or stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    Continue,
    Stop,
}

// ---

pub fn run<H, C>(mut paths: Vec<PathBuf>, mut handle: H, is_cancelled: C) -> Result<()>
where
    H: FnMut(Event) -> Result<ControlFlow>,
    C: Fn() -> bool,
{
    log::debug!("fsmon::run: starting with {} paths", paths.len());
    if paths.is_empty() {
        log::debug!("fsmon::run: no paths, returning early");
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

    log::debug!("fsmon::run: entering imp::run with {} watch paths", watch.len());
    imp::run(
        watch,
        |event| {
            log::debug!("fsmon::run: received event: {:?}", event.kind);
            if event.paths.iter().any(|path| paths.binary_search(path).is_ok()) {
                handle(event)
            } else {
                Ok(ControlFlow::Continue)
            }
        },
        &is_cancelled,
    )
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::sync::mpsc::{self, RecvTimeoutError};

    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    use super::*;
    use crate::error::Error;

    const FALLBACK_POLLING_INTERVAL: Duration = Duration::from_secs(1);
    const CANCELLATION_CHECK_INTERVAL: Duration = Duration::from_millis(50);

    pub fn run<H, C>(paths: Vec<PathBuf>, mut handle: H, is_cancelled: C) -> Result<()>
    where
        H: FnMut(Event) -> Result<ControlFlow>,
        C: Fn() -> bool,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

        for path in &paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        loop {
            if is_cancelled() {
                log::debug!("fsmon::imp::run: cancelled, exiting");
                return Ok(());
            }

            match rx.recv_timeout(CANCELLATION_CHECK_INTERVAL) {
                Ok(Ok(event)) => {
                    if handle(event)? == ControlFlow::Stop {
                        log::debug!("fsmon::imp::run: handler requested stop");
                        return Ok(());
                    }
                }
                Ok(Err(err)) => return Err(err.into()),
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(Error::RecvTimeoutError {
                        source: RecvTimeoutError::Disconnected.into(),
                    });
                }
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

    const CANCELLATION_CHECK_INTERVAL: Duration = Duration::from_millis(50);

    pub fn run<H, C>(paths: Vec<PathBuf>, mut handle: H, is_cancelled: C) -> Result<()>
    where
        H: FnMut(Event) -> Result<ControlFlow>,
        C: Fn() -> bool,
    {
        log::debug!("fsmon::imp::run (macos): starting with {} paths", paths.len());
        let mut watcher = Watcher::new()?;
        let mut added = HashSet::<&PathBuf>::new();
        let mut synced = true;

        let flags = FilterFlag::NOTE_FFNOP
            | FilterFlag::NOTE_DELETE
            | FilterFlag::NOTE_WRITE
            | FilterFlag::NOTE_RENAME
            | FilterFlag::NOTE_EXTEND;

        loop {
            if is_cancelled() {
                log::debug!("fsmon::imp::run (macos): cancelled, exiting");
                return Ok(());
            }

            log::debug!("fsmon::imp::run (macos): outer loop iteration, synced={}", synced);
            for path in &paths {
                if watcher.add_filename(path, EventFilter::EVFILT_VNODE, flags).is_ok() {
                    added.insert(path);
                    if !synced {
                        log::debug!("fsmon::imp::run (macos): calling handle for path {:?}", path);
                        if handle(Event::new(EventKind::Create(CreateKind::Any)).add_path(path.clone()))?
                            == ControlFlow::Stop
                        {
                            log::debug!("fsmon::imp::run (macos): handler requested stop");
                            return Ok(());
                        }
                    }
                }
            }

            synced = true;
            log::debug!("fsmon::imp::run (macos): calling watcher.watch()");
            watcher.watch()?;

            while synced {
                if is_cancelled() {
                    log::debug!("fsmon::imp::run (macos): cancelled in inner loop, exiting");
                    return Ok(());
                }

                let event = if let Some(event) = watcher.poll(Some(CANCELLATION_CHECK_INTERVAL)) {
                    event
                } else {
                    log::trace!("fsmon::imp::run (macos): poll timeout, continuing");
                    continue;
                };

                log::debug!("fsmon::imp::run (macos): received kqueue event");
                match event {
                    kqueue::Event {
                        data: EventData::Vnode(data),
                        ident: Ident::Filename(_, path),
                    } => {
                        let path = PathBuf::from(path);
                        log::debug!("fsmon::imp::run (macos): vnode event {:?} for {:?}", data, path);
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
                        log::debug!("fsmon::imp::run (macos): calling handle for event");
                        if handle(event)? == ControlFlow::Stop {
                            log::debug!("fsmon::imp::run (macos): handler requested stop");
                            return Ok(());
                        }
                    }
                    _ => unreachable!(),
                };
            }
        }
    }
}
