use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
use notify::{Config, Event, EventKind, PollWatcher, RecursiveMode, Watcher};

use crate::Error;

use super::{Backend, WakeHint, WakeSignal};

// ---

pub struct PollBackend {
    watcher: PollWatcher,
    path_to_idx: Arc<Mutex<HashMap<PathBuf, usize>>>,
}

impl PollBackend {
    pub fn new(tx: SyncSender<WakeSignal>, poll_interval: Duration) -> Result<Self, Error> {
        let (ntx, nrx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let path_to_idx: Arc<Mutex<HashMap<PathBuf, usize>>> = Arc::new(Mutex::new(HashMap::new()));
        let idx_map = path_to_idx.clone();

        thread::Builder::new()
            .name("fsmon-poll".into())
            .spawn(move || {
                for event in nrx.into_iter().flatten() {
                    let hint = event_to_hint(&event);
                    let map = idx_map.lock().unwrap();
                    for path in &event.paths {
                        if let Some(&idx) = map.get(path) {
                            let _ = tx.try_send(WakeSignal { source_idx: idx, hint });
                        }
                    }
                }
            })
            .map_err(Error::Io)?;

        let watcher =
            PollWatcher::new(ntx, Config::default().with_poll_interval(poll_interval)).map_err(Error::Watch)?;

        Ok(Self { watcher, path_to_idx })
    }
}

impl Backend for PollBackend {
    fn watch(&mut self, path: PathBuf, source_idx: usize) -> Result<(), Error> {
        self.watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(Error::Watch)?;
        self.path_to_idx.lock().unwrap().insert(path, source_idx);
        Ok(())
    }

    fn unwatch(&mut self, path: &Path) -> Result<(), Error> {
        self.watcher.unwatch(path).map_err(Error::Watch)?;
        self.path_to_idx.lock().unwrap().remove(path);
        Ok(())
    }
}

fn event_to_hint(event: &Event) -> WakeHint {
    match event.kind {
        EventKind::Create(CreateKind::File | CreateKind::Any) => WakeHint::Created,
        EventKind::Remove(RemoveKind::File | RemoveKind::Any) => WakeHint::Removed,
        EventKind::Modify(ModifyKind::Name(RenameMode::From | RenameMode::Any)) => WakeHint::Removed,
        _ => WakeHint::Data,
    }
}
