use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::time::Duration;

use crate::classify::{Reliability, classify};
use crate::options::{FallbackPolicy, FollowOptions};
use crate::{Error, Result, SourceId};

use super::Event;
use super::backend::{self, Backend, WakeSignal};

// ---

/// The mechanism currently serving a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Native,
    Polling,
}

// ---

struct SourceEntry {
    #[allow(dead_code)]
    path: PathBuf,
    engine: Engine,
}

// ---

pub(crate) struct EngineState {
    sources: Vec<SourceEntry>,
    rx: mpsc::Receiver<WakeSignal>,
    native_backend: Option<Box<dyn Backend>>,
    poll_backend: Option<Box<dyn Backend>>,
    options: FollowOptions,
}

impl EngineState {
    pub fn new<I>(paths: I, options: FollowOptions) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        let (tx, rx) = mpsc::sync_channel::<WakeSignal>(256);

        let mut state = Self {
            sources: Vec::new(),
            rx,
            native_backend: None,
            poll_backend: None,
            options: options.clone(),
        };

        for path in paths {
            state.add_path(path.as_ref(), tx.clone())?;
        }

        Ok(state)
    }

    fn add_path(&mut self, path: &Path, tx: SyncSender<WakeSignal>) -> Result<()> {
        let idx = self.sources.len();
        let reliability = classify(path);

        let use_native = match self.options.fallback_policy {
            FallbackPolicy::Conservative => reliability == Reliability::KnownLocal,
            FallbackPolicy::Optimistic => true,
        };

        let engine = if use_native {
            log::debug!("fsmon: {} classified {:?} -> native", path.display(), reliability);
            // Ensure native backend exists.
            if self.native_backend.is_none() {
                self.native_backend = Some(backend::new_native(tx.clone(), self.options.poll_interval)?);
            }
            if let Some(nb) = &mut self.native_backend {
                nb.watch(path.to_path_buf(), idx)?;
            }
            Engine::Native
        } else {
            log::debug!(
                "fsmon: {} classified {:?} -> polling (conservative fallback)",
                path.display(),
                reliability
            );
            // Ensure poll backend exists.
            if self.poll_backend.is_none() {
                self.poll_backend = Some(backend::new_poll(tx.clone(), self.options.poll_interval)?);
            }
            if let Some(pb) = &mut self.poll_backend {
                pb.watch(path.to_path_buf(), idx)?;
            }
            Engine::Polling
        };

        self.sources.push(SourceEntry {
            path: path.to_path_buf(),
            engine,
        });

        Ok(())
    }

    /// Degrade a source from native to polling after a watch loss (FR-014).
    #[allow(dead_code)]
    pub fn degrade_to_polling(&mut self, source_idx: usize, tx: SyncSender<WakeSignal>) {
        if source_idx >= self.sources.len() {
            return;
        }
        let entry = &mut self.sources[source_idx];
        if entry.engine == Engine::Native {
            log::warn!(
                "fsmon: {} native watch lost, migrating to polling",
                entry.path.display()
            );
            entry.engine = Engine::Polling;
            // Unwatch from native backend.
            if let Some(nb) = &mut self.native_backend {
                let _ = nb.unwatch(&entry.path.clone());
            }
            // Add to poll backend.
            if self.poll_backend.is_none() {
                self.poll_backend = backend::new_poll(tx.clone(), self.options.poll_interval).ok();
            }
            if let Some(pb) = &mut self.poll_backend {
                let _ = pb.watch(entry.path.clone(), source_idx);
            }
        }
    }

    pub fn engine_for(&self, source: SourceId) -> Engine {
        self.sources.get(source.0).map(|e| e.engine).unwrap_or(Engine::Polling)
    }

    pub fn tick_interval(&self) -> Duration {
        self.options.recheck_cadence.min(self.options.poll_interval)
    }

    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    pub fn recv_timeout(&mut self, timeout: Duration) -> std::result::Result<WakeSignal, RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }

    /// Convert a raw wake signal into an advisory `Event`.
    pub fn recv_event(&mut self) -> Result<Event> {
        let interval = self.tick_interval();
        loop {
            match self.rx.recv_timeout(interval) {
                Ok(signal) => {
                    let source = SourceId(signal.source_idx);
                    let event = match signal.hint {
                        backend::WakeHint::Data => Event::DataAvailable(source),
                        backend::WakeHint::Removed => Event::Removed(source),
                        backend::WakeHint::Created => Event::Reappeared(source),
                    };
                    return Ok(event);
                }
                Err(RecvTimeoutError::Timeout) => {
                    // Tick — emit a DataAvailable for source 0 to drive reconcile.
                    // The facade reconciles all sources on tick.
                    if !self.sources.is_empty() {
                        return Ok(Event::DataAvailable(SourceId(0)));
                    }
                    // No sources — spin.
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "watch backend stopped",
                    )));
                }
            }
        }
    }
}
