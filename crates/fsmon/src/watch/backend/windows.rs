// Windows hybrid backend: ReadDirectoryChangesW (via notify) + adaptive
// GetFileInformationByHandle polling.
//
// Windows NTFS defers directory index entry updates (size, mtime) until
// CloseHandle or FlushFileBuffers.  ReadDirectoryChangesW monitors the
// directory index, so it misses writes from processes that keep the file
// open without flushing — the common case for log writers like lumberjack.
//
// There is no non-elevated Windows API that fires immediately on WriteFile
// while the writer holds the handle open.  GetFileInformationByHandle reads
// the live MFT record directly, bypassing the directory cache, so polling
// it is the only reliable write-detection mechanism available without
// elevation.  FSCTL_READ_USN_JOURNAL would be event-driven but requires
// SE_MANAGE_VOLUME_PRIVILEGE.
//
// Strategy: ReadDirectoryChangesW for instant rotation/deletion/creation
// events, plus adaptive GetFileInformationByHandle polling for write
// detection.
//
// Ported from src/fsmon/windows.rs (commit 3c8e6162) and integrated into the
// crates/fsmon backend architecture.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::Error;
use crate::identity::{self, FileId};
use crate::watch::interval::{IntervalEstimator, min_sleep_duration};

// ---

use super::{Backend, WakeHint, WakeSignal};

// ---

enum Seen {
    Never,
    Present { id: FileId, size: u64 },
    Absent,
}

struct SourceState {
    path: PathBuf,
    source_idx: usize,
    seen: Seen,
    estimator: IntervalEstimator,
}

enum Cmd {
    Add { path: PathBuf, source_idx: usize },
    Remove { path: PathBuf },
}

// ---

pub struct WindowsNativeBackend {
    watcher: RecommendedWatcher,
    path_to_idx: Arc<Mutex<HashMap<PathBuf, usize>>>,
    cmd_tx: mpsc::Sender<Cmd>,
}

impl WindowsNativeBackend {
    pub fn new(tx: SyncSender<WakeSignal>) -> Result<Self, Error> {
        let path_to_idx: Arc<Mutex<HashMap<PathBuf, usize>>> = Arc::new(Mutex::new(HashMap::new()));
        let idx_map = path_to_idx.clone();

        // Thread 1: ReadDirectoryChangesW via notify.
        // Catches file rotation (rename/create/delete) immediately; write events
        // may also arrive whenever the NTFS lazy writer syncs the directory entry.
        let (ntx, nrx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        {
            let tx = tx.clone();
            thread::Builder::new()
                .name("fsmon-win-native".into())
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
        }

        // Thread 2: adaptive GetFileInformationByHandle polling.
        // Opens a fresh handle per tick to read the live MFT record, catching
        // writes to open files that ReadDirectoryChangesW would miss.
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        {
            let tx = tx.clone();
            thread::Builder::new()
                .name("fsmon-win-poll".into())
                .spawn(move || run_poll_loop(tx, cmd_rx))
                .map_err(Error::Io)?;
        }

        let watcher = RecommendedWatcher::new(ntx, Config::default()).map_err(Error::Watch)?;

        Ok(Self {
            watcher,
            path_to_idx,
            cmd_tx,
        })
    }
}

impl Backend for WindowsNativeBackend {
    fn watch(&mut self, path: PathBuf, source_idx: usize) -> Result<(), Error> {
        self.watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(Error::Watch)?;
        self.path_to_idx.lock().unwrap().insert(path.clone(), source_idx);
        self.cmd_tx
            .send(Cmd::Add { path, source_idx })
            .map_err(|_| broken_pipe())?;
        Ok(())
    }

    fn unwatch(&mut self, path: &Path) -> Result<(), Error> {
        self.watcher.unwatch(path).map_err(Error::Watch)?;
        self.path_to_idx.lock().unwrap().remove(path);
        self.cmd_tx
            .send(Cmd::Remove {
                path: path.to_path_buf(),
            })
            .map_err(|_| broken_pipe())?;
        Ok(())
    }
}

// ---

fn run_poll_loop(tx: SyncSender<WakeSignal>, cmd_rx: mpsc::Receiver<Cmd>) {
    let mut sources: Vec<SourceState> = Vec::new();

    loop {
        if sources.is_empty() {
            match cmd_rx.recv() {
                Ok(cmd) => apply_cmd(&mut sources, cmd),
                Err(_) => return,
            }
            continue;
        }

        let sleep = min_sleep_duration(sources.iter().map(|s| &s.estimator));
        thread::sleep(sleep);

        drain_cmds(&mut sources, &cmd_rx);
        if sources.is_empty() {
            continue;
        }

        for state in &mut sources {
            tick_source(state, &tx);
        }
    }
}

fn tick_source(state: &mut SourceState, tx: &SyncSender<WakeSignal>) {
    // Open with FILE_SHARE_DELETE so we can query files mid-rotation.
    // From the open handle, read both identity (via GetFileInformationByHandleEx
    // or its fallback) and live size (via GetFileInformationByHandle through
    // file.metadata()) — both bypass the NTFS directory cache.
    let probe = identity::open_shared(&state.path).ok().and_then(|f| {
        let id = FileId::from_file(&f).ok()?;
        let size = f.metadata().ok()?.len();
        Some((id, size))
    });
    match probe {
        Some((current_id, size)) => {
            let rotated = matches!(&state.seen, Seen::Present { id, .. } if *id != current_id);
            let appeared = matches!(&state.seen, Seen::Absent);
            let changed = match &state.seen {
                Seen::Never => false,
                Seen::Absent => true,
                Seen::Present { id, size: prev_size } => *id != current_id || *prev_size != size,
            };
            state.seen = Seen::Present { id: current_id, size };
            if changed {
                state.estimator.on_change();
                if rotated {
                    let _ = tx.try_send(WakeSignal {
                        source_idx: state.source_idx,
                        hint: WakeHint::Removed,
                    });
                    let _ = tx.try_send(WakeSignal {
                        source_idx: state.source_idx,
                        hint: WakeHint::Created,
                    });
                } else {
                    let hint = if appeared { WakeHint::Created } else { WakeHint::Data };
                    let _ = tx.try_send(WakeSignal {
                        source_idx: state.source_idx,
                        hint,
                    });
                }
            } else {
                state.estimator.on_no_change();
            }
        }
        None => {
            if matches!(&state.seen, Seen::Present { .. }) {
                state.seen = Seen::Absent;
                state.estimator.on_change();
                let _ = tx.try_send(WakeSignal {
                    source_idx: state.source_idx,
                    hint: WakeHint::Removed,
                });
            } else {
                state.estimator.on_no_change();
            }
        }
    }
}

fn apply_cmd(sources: &mut Vec<SourceState>, cmd: Cmd) {
    match cmd {
        Cmd::Add { path, source_idx } => {
            if !sources.iter().any(|s| s.path == path) {
                sources.push(SourceState {
                    path,
                    source_idx,
                    seen: Seen::Never,
                    estimator: IntervalEstimator::new(),
                });
            }
        }
        Cmd::Remove { path } => {
            sources.retain(|s| s.path != path);
        }
    }
}

fn drain_cmds(sources: &mut Vec<SourceState>, cmd_rx: &mpsc::Receiver<Cmd>) {
    loop {
        match cmd_rx.try_recv() {
            Ok(cmd) => apply_cmd(sources, cmd),
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
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

fn broken_pipe() -> Error {
    Error::Io(std::io::Error::new(
        std::io::ErrorKind::BrokenPipe,
        "windows poll thread stopped",
    ))
}
