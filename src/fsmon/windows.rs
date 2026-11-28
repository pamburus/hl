use std::{
    path::PathBuf,
    sync::{Arc, Weak},
    thread,
    time::{Duration, Instant},
};

use notify::{
    Config, RecommendedWatcher, RecursiveMode, Watcher,
    event::{CreateKind, DataChange, ModifyKind, RemoveKind},
};

use super::{Event, EventKind, Filter, queue::Queue};
use crate::{error::Result, win_file_id};

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

pub(super) fn run<H>(paths: Vec<PathBuf>, filter: Filter, mut handle: H) -> Result<()>
where
    H: FnMut(Event) -> Result<()>,
{
    let queue = Arc::new(Queue::new(filter));

    // ReadDirectoryChangesW via notify.
    // Catches file rotation (rename/create/delete) immediately; write events
    // may also arrive whenever the NTFS lazy writer syncs the directory entry.
    // Errors are dropped, as the polling below still detects the changes.
    let push = {
        let queue = queue.clone();
        move |event: notify::Result<Event>| {
            if event.is_ok() {
                queue.push(event);
            }
        }
    };
    let mut watcher = RecommendedWatcher::new(push, Config::default())?;
    for path in &paths {
        watcher.watch(path, RecursiveMode::NonRecursive)?;
    }

    // Adaptive GetFileInformationByHandle polling in a thread of its own,
    // which stops once the queue is gone.
    // Opens a fresh handle per tick to read the live MFT record, catching
    // writes to open files that ReadDirectoryChangesW would miss.
    {
        let queue = Arc::downgrade(&queue);
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

            let send =
                |queue: &Arc<Queue>, kind, path: &PathBuf| queue.push(Ok(Event::new(kind).add_path(path.clone())));

            loop {
                thread::sleep(sleep_duration(states.iter().map(|s| s.estimator.avg_secs)));
                let Some(queue) = Weak::upgrade(&queue) else { return };
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
                                    send(&queue, EventKind::Remove(RemoveKind::File), &state.path);
                                    send(&queue, EventKind::Create(CreateKind::File), &state.path);
                                } else {
                                    let kind = if appeared {
                                        EventKind::Create(CreateKind::File)
                                    } else {
                                        EventKind::Modify(ModifyKind::Data(DataChange::Size))
                                    };
                                    send(&queue, kind, &state.path);
                                }
                            } else {
                                state.estimator.on_no_change();
                            }
                        }
                        None => {
                            if matches!(state.seen, Seen::Present { .. }) {
                                state.seen = Seen::Absent;
                                state.estimator.on_change();
                                send(&queue, EventKind::Remove(RemoveKind::File), &state.path);
                            } else {
                                state.estimator.on_no_change();
                            }
                        }
                    }
                }
            }
        });
    }

    loop {
        handle(queue.pop()?)?;
    }
}
