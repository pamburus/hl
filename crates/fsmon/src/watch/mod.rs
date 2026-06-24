use std::path::Path;
use std::time::Duration;

use crate::options::FollowOptions;
use crate::{Result, SourceId};

// ---

pub(crate) mod backend;
pub mod engine;
pub(crate) mod interval;

pub use engine::Engine;

// ---

/// Advisory event emitted by the core `Watcher`. The facade always
/// re-derives truth from `stat` and does not rely on event accuracy for
/// correctness (research §2).
#[derive(Debug, Clone)]
pub enum Event {
    DataAvailable(SourceId),
    Rotated(SourceId),
    Truncated(SourceId),
    Removed(SourceId),
    Reappeared(SourceId),
}

// ---

/// The core event-level watcher. Exposes per-path engine selection and
/// advisory events. Advanced consumers (tail-style messages, OOS-001) can
/// use this directly; most callers use the `Follower` facade instead.
pub struct Watcher {
    engine: engine::EngineState,
}

impl Watcher {
    pub fn new<I>(paths: I, options: FollowOptions) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        let engine = engine::EngineState::new(paths, options)?;
        Ok(Self { engine })
    }

    /// Block for the next advisory event. The recheck-cadence tick is
    /// transparent to this API (it drives internal reconciliation without
    /// surfacing as an `Event`).
    pub fn recv(&mut self) -> Result<Event> {
        self.engine.recv_event()
    }

    /// The current engine for a source (for diagnostics and tests).
    pub fn engine(&self, source: SourceId) -> Engine {
        self.engine.engine_for(source)
    }

    /// Timeout to use in the facade's `recv_timeout` loop (the minimum
    /// interval across all configured cadences).
    pub(crate) fn tick_interval(&self) -> Duration {
        self.engine.tick_interval()
    }

    /// Receive a raw wake signal (used by the Follower facade).
    pub(crate) fn recv_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::result::Result<backend::WakeSignal, std::sync::mpsc::RecvTimeoutError> {
        self.engine.recv_timeout(timeout)
    }

    #[allow(dead_code)]
    pub(crate) fn source_count(&self) -> usize {
        self.engine.source_count()
    }
}
