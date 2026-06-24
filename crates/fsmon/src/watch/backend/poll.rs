// Adaptive polling backend — cross-platform fallback for paths on unreliable
// filesystems (network shares, FUSE mounts, or any path whose reliability
// cannot be confirmed).
//
// Unlike a fixed-interval poll, this backend uses an IIR-based IntervalEstimator
// per source so that actively-written files are checked at near-event latency
// (≥10 ms) while idle files relax to a configurable ceiling (default 1 s).
// The estimator is the same one used by the Windows hybrid backend so that
// adaptive behaviour is consistent across all polling paths.
//
// Change detection uses fs::metadata — size-only comparison is sufficient
// because same-size replacements are caught by FollowedSource's recheck_cadence
// tick regardless of which backend is in use.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::Duration;

use crate::Error;

use super::{Backend, WakeHint, WakeSignal};
use crate::watch::interval::{IntervalEstimator, min_sleep_duration};

// ---

struct SourceState {
    source_idx: usize,
    /// None = first observation (no baseline yet; no event sent on initial tick).
    last_size: Option<u64>,
    was_present: bool,
    estimator: IntervalEstimator,
}

enum Cmd {
    Add { path: PathBuf, source_idx: usize },
    Remove { path: PathBuf },
}

// ---

pub struct PollBackend {
    cmd_tx: mpsc::Sender<Cmd>,
}

impl PollBackend {
    pub fn new(tx: SyncSender<WakeSignal>, _poll_interval: Duration) -> Result<Self, Error> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();

        thread::Builder::new()
            .name("fsmon-poll".into())
            .spawn(move || run_poll_loop(tx, cmd_rx))
            .map_err(Error::Io)?;

        Ok(Self { cmd_tx })
    }
}

impl Backend for PollBackend {
    fn watch(&mut self, path: PathBuf, source_idx: usize) -> Result<(), Error> {
        self.cmd_tx
            .send(Cmd::Add { path, source_idx })
            .map_err(|_| broken_pipe())
    }

    fn unwatch(&mut self, path: &Path) -> Result<(), Error> {
        self.cmd_tx
            .send(Cmd::Remove {
                path: path.to_path_buf(),
            })
            .map_err(|_| broken_pipe())
    }
}

// ---

fn run_poll_loop(tx: SyncSender<WakeSignal>, cmd_rx: mpsc::Receiver<Cmd>) {
    let mut sources: Vec<(PathBuf, SourceState)> = Vec::new();

    loop {
        if sources.is_empty() {
            // No sources — block on the command channel to avoid busy-waiting.
            match cmd_rx.recv() {
                Ok(cmd) => apply_cmd(&mut sources, cmd),
                Err(_) => return,
            }
            continue;
        }

        // Adaptive sleep: half the minimum average interval across all sources.
        let sleep = min_sleep_duration(sources.iter().map(|(_, s)| &s.estimator));
        thread::sleep(sleep);

        // Drain any pending commands before polling.
        drain_cmds(&mut sources, &cmd_rx);

        // Poll each source.
        for (path, state) in &mut sources {
            tick_source(path, state, &tx);
        }
    }
}

fn tick_source(path: &Path, state: &mut SourceState, tx: &SyncSender<WakeSignal>) {
    match std::fs::metadata(path) {
        Ok(meta) => {
            let size = meta.len();
            match state.last_size {
                None => {
                    // First observation: record baseline, no event.
                    state.last_size = Some(size);
                    state.was_present = true;
                }
                Some(prev) => {
                    let appeared = !state.was_present;
                    state.was_present = true;
                    state.last_size = Some(size);

                    if appeared {
                        state.estimator.on_change();
                        let _ = tx.try_send(WakeSignal {
                            source_idx: state.source_idx,
                            hint: WakeHint::Created,
                        });
                    } else if size != prev {
                        state.estimator.on_change();
                        let _ = tx.try_send(WakeSignal {
                            source_idx: state.source_idx,
                            hint: WakeHint::Data,
                        });
                    } else {
                        state.estimator.on_no_change();
                    }
                }
            }
        }
        Err(_) => {
            if state.was_present {
                state.was_present = false;
                state.last_size = None;
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

fn apply_cmd(sources: &mut Vec<(PathBuf, SourceState)>, cmd: Cmd) {
    match cmd {
        Cmd::Add { path, source_idx } => {
            // Only add if not already tracked.
            if !sources.iter().any(|(p, _)| p == &path) {
                sources.push((
                    path,
                    SourceState {
                        source_idx,
                        last_size: None,
                        was_present: false,
                        estimator: IntervalEstimator::new(),
                    },
                ));
            }
        }
        Cmd::Remove { path } => {
            sources.retain(|(p, _)| p != &path);
        }
    }
}

fn drain_cmds(sources: &mut Vec<(PathBuf, SourceState)>, cmd_rx: &mpsc::Receiver<Cmd>) {
    loop {
        match cmd_rx.try_recv() {
            Ok(cmd) => apply_cmd(sources, cmd),
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }
}

fn broken_pipe() -> Error {
    Error::Io(std::io::Error::new(
        std::io::ErrorKind::BrokenPipe,
        "poll backend stopped",
    ))
}
