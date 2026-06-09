use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::Duration;

use kqueue::{EventData, EventFilter, FilterFlag, Ident, Vnode, Watcher as KqWatcher};

use crate::Error;

use super::{Backend, WakeHint, WakeSignal};

// ---

const POLL_TIMEOUT: Duration = Duration::from_secs(1);

// ---

enum KqueueCmd {
    Watch(PathBuf, usize),
    #[allow(dead_code)]
    Unwatch(PathBuf),
}

// ---

/// macOS kqueue-based native backend. Gated to `KnownLocal` paths only
/// (FR-020). The background thread runs the existing kqueue loop adapted from
/// `src/fsmon.rs`.
pub struct KqueueBackend {
    cmd_tx: mpsc::Sender<KqueueCmd>,
}

impl KqueueBackend {
    pub fn new(tx: SyncSender<WakeSignal>) -> Result<Self, Error> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<KqueueCmd>();

        thread::Builder::new()
            .name("fsmon-kqueue".into())
            .spawn(move || run_kqueue_loop(tx, cmd_rx))
            .map_err(Error::Io)?;

        Ok(Self { cmd_tx })
    }
}

impl Backend for KqueueBackend {
    fn watch(&mut self, path: PathBuf, source_idx: usize) -> Result<(), Error> {
        self.cmd_tx
            .send(KqueueCmd::Watch(path, source_idx))
            .map_err(|_| Error::Io(std::io::Error::other("kqueue backend stopped")))
    }

    fn unwatch(&mut self, path: &Path) -> Result<(), Error> {
        self.cmd_tx
            .send(KqueueCmd::Unwatch(path.to_path_buf()))
            .map_err(|_| Error::Io(std::io::Error::other("kqueue backend stopped")))
    }
}

// ---

fn run_kqueue_loop(tx: SyncSender<WakeSignal>, cmd_rx: mpsc::Receiver<KqueueCmd>) {
    let Ok(mut watcher) = KqWatcher::new() else {
        return;
    };

    let flags = FilterFlag::NOTE_FFNOP
        | FilterFlag::NOTE_DELETE
        | FilterFlag::NOTE_WRITE
        | FilterFlag::NOTE_RENAME
        | FilterFlag::NOTE_EXTEND;

    let mut path_to_idx: HashMap<PathBuf, usize> = HashMap::new();
    let mut watched: HashSet<PathBuf> = HashSet::new();

    loop {
        // Drain all pending commands without blocking.
        loop {
            match cmd_rx.try_recv() {
                Ok(KqueueCmd::Watch(path, idx)) => {
                    path_to_idx.insert(path, idx);
                }
                Ok(KqueueCmd::Unwatch(path)) => {
                    path_to_idx.remove(&path);
                    if watched.remove(&path) {
                        let _ = watcher.remove_filename(&path, EventFilter::EVFILT_VNODE);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }

        // Try to add any unwatched paths.
        let mut just_added = false;
        for (path, &idx) in &path_to_idx {
            if !watched.contains(path) && watcher.add_filename(path, EventFilter::EVFILT_VNODE, flags).is_ok() {
                watched.insert(path.clone());
                just_added = true;
                // Emit a wake so the source is reconciled after reappearance.
                let _ = tx.try_send(WakeSignal {
                    source_idx: idx,
                    hint: WakeHint::Created,
                });
            }
        }

        if just_added && watcher.watch().is_err() {
            break;
        }

        // Poll for events (non-blocking if no paths are watched).
        if watched.is_empty() {
            // No paths registered yet — sleep briefly then re-check commands.
            std::thread::sleep(POLL_TIMEOUT);
            continue;
        }

        // Poll with a short timeout to also process new commands promptly.
        while let Some(event) = watcher.poll(Some(POLL_TIMEOUT)) {
            if let kqueue::Event {
                data: EventData::Vnode(data),
                ident: Ident::Filename(_, path),
            } = event
            {
                let path = PathBuf::from(path);
                let idx = match path_to_idx.get(&path) {
                    Some(&i) => i,
                    None => continue,
                };

                let hint = match data {
                    Vnode::Delete | Vnode::Revoke => {
                        if watched.remove(&path) {
                            let _ = watcher.remove_filename(&path, EventFilter::EVFILT_VNODE);
                        }
                        WakeHint::Removed
                    }
                    Vnode::Rename => {
                        if watched.remove(&path) {
                            let _ = watcher.remove_filename(&path, EventFilter::EVFILT_VNODE);
                        }
                        WakeHint::Removed
                    }
                    _ => WakeHint::Data,
                };

                let _ = tx.try_send(WakeSignal { source_idx: idx, hint });
            }
        }
    }
}
