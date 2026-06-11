use std::{
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use notify::{
    Config, RecommendedWatcher, RecursiveMode, Watcher,
    event::{CreateKind, DataChange, ModifyKind, RemoveKind},
};

use super::{Event, EventKind};
use crate::{
    error::{Error, Result},
    win_file_id,
};

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

const IIR_ALPHA: f64 = 0.25;
const FLOOR_SECS: f64 = 0.010; // 10 ms
const DEFAULT_SECS: f64 = 0.100; // 100 ms → initial sleep of 50 ms
const CEILING_SECS: f64 = 1.0; // 1 s

// Tracks the EMA of the interval between observed updates for a single file.
struct IntervalEstimator {
    avg_secs: f64,
    last_change: Option<Instant>,
}

impl IntervalEstimator {
    fn new() -> Self {
        Self {
            avg_secs: DEFAULT_SECS,
            last_change: None,
        }
    }

    fn on_change(&mut self) {
        let now = Instant::now();
        if let Some(prev) = self.last_change.replace(now) {
            let dt = now.duration_since(prev).as_secs_f64();
            self.avg_secs = IIR_ALPHA * dt + (1.0 - IIR_ALPHA) * self.avg_secs;
        }
    }

    // When idle time exceeds the current average, drift the estimate upward
    // so that a quiet file relaxes toward the ceiling rather than keeping
    // the shared sleep interval pinned at its last active rate.
    fn on_no_change(&mut self) {
        let Some(last) = self.last_change else { return };
        let idle = last.elapsed().as_secs_f64();
        if idle > self.avg_secs {
            self.avg_secs = IIR_ALPHA * idle + (1.0 - IIR_ALPHA) * self.avg_secs;
            // Cap so recovery from a long idle period isn't too slow.
            self.avg_secs = self.avg_secs.min(CEILING_SECS * 2.0);
        }
    }
}

fn sleep_duration(avg_secs: impl Iterator<Item = f64>) -> Duration {
    let min = avg_secs.fold(f64::INFINITY, f64::min);
    Duration::from_secs_f64((min / 2.0).clamp(FLOOR_SECS, CEILING_SECS))
}

pub(super) fn run<H>(paths: Vec<PathBuf>, mut handle: H) -> Result<()>
where
    H: FnMut(Event) -> Result<()>,
{
    let (tx, rx) = mpsc::channel::<Event>();

    // Thread 1: ReadDirectoryChangesW via notify.
    // Catches file rotation (rename/create/delete) immediately; write events
    // may also arrive whenever the NTFS lazy writer syncs the directory entry.
    let (wtx, wrx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(wtx, Config::default())?;
    for path in &paths {
        watcher.watch(path, RecursiveMode::NonRecursive)?;
    }
    {
        let tx = tx.clone();
        thread::spawn(move || {
            let _watcher = watcher;
            while let Ok(Ok(event)) = wrx.recv() {
                if tx.send(event).is_err() {
                    break;
                }
            }
        });
    }

    // Thread 2: adaptive GetFileInformationByHandle polling.
    // Opens a fresh handle per tick to read the live MFT record, catching
    // writes to open files that ReadDirectoryChangesW would miss.
    {
        let tx = tx.clone();
        let file_paths: Vec<PathBuf> = paths.into_iter().filter(|p| !p.is_dir()).collect();
        thread::spawn(move || {
            enum Seen {
                Never,
                Present { id: win_file_id::FileId, size: u64 },
                Absent,
            }
            struct State {
                path: PathBuf,
                seen: Seen,
                estimator: IntervalEstimator,
            }
            let mut states: Vec<State> = file_paths
                .into_iter()
                .map(|path| State {
                    path,
                    seen: Seen::Never,
                    estimator: IntervalEstimator::new(),
                })
                .collect();

            loop {
                thread::sleep(sleep_duration(states.iter().map(|s| s.estimator.avg_secs)));
                for state in &mut states {
                    match win_file_id::open_shared(&state.path).and_then(|f| win_file_id::query(&f)) {
                        Some(info) => {
                            let rotated = matches!(&state.seen, Seen::Present { id, .. } if *id != info.id);
                            let appeared = matches!(&state.seen, Seen::Absent);
                            let changed = match &state.seen {
                                Seen::Never => false,
                                Seen::Absent => true,
                                Seen::Present { id, size } => *id != info.id || *size != info.size,
                            };
                            state.seen = Seen::Present {
                                id: info.id,
                                size: info.size,
                            };
                            if changed {
                                state.estimator.on_change();
                                if rotated {
                                    if tx
                                        .send(
                                            Event::new(EventKind::Remove(RemoveKind::File))
                                                .add_path(state.path.clone()),
                                        )
                                        .is_err()
                                    {
                                        return;
                                    }
                                    if tx
                                        .send(
                                            Event::new(EventKind::Create(CreateKind::File))
                                                .add_path(state.path.clone()),
                                        )
                                        .is_err()
                                    {
                                        return;
                                    }
                                } else {
                                    let kind = if appeared {
                                        EventKind::Create(CreateKind::File)
                                    } else {
                                        EventKind::Modify(ModifyKind::Data(DataChange::Size))
                                    };
                                    if tx.send(Event::new(kind).add_path(state.path.clone())).is_err() {
                                        return;
                                    }
                                }
                            } else {
                                state.estimator.on_no_change();
                            }
                        }
                        None => {
                            if matches!(state.seen, Seen::Present { .. }) {
                                state.seen = Seen::Absent;
                                state.estimator.on_change();
                                if tx
                                    .send(Event::new(EventKind::Remove(RemoveKind::File)).add_path(state.path.clone()))
                                    .is_err()
                                {
                                    return;
                                }
                            } else {
                                state.estimator.on_no_change();
                            }
                        }
                    }
                }
            }
        });
    }

    drop(tx);

    loop {
        match rx.recv() {
            Ok(event) => handle(event)?,
            Err(err) => return Err(Error::RecvTimeoutError { source: err.into() }),
        }
    }
}
