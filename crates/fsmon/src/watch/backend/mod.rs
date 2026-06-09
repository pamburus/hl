use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use crate::Error;

// ---

pub mod native;
pub mod poll;

#[cfg(target_os = "macos")]
pub mod kqueue_macos;

// ---

/// A raw wake signal from a backend: the index of the source that changed,
/// and an advisory hint about what happened. The facade always re-derives
/// truth from `stat` and does not rely on the hint for correctness.
#[derive(Debug, Clone)]
pub struct WakeSignal {
    /// Index into the `Follower`'s source list.
    pub source_idx: usize,
    pub hint: WakeHint,
}

#[derive(Debug, Clone, Copy)]
pub enum WakeHint {
    /// Data may have been appended, or a general filesystem event.
    Data,
    /// Path was removed or renamed.
    Removed,
    /// Path was created (reappearance after removal).
    Created,
}

// ---

/// Abstraction over a filesystem notification backend.
pub trait Backend: Send + 'static {
    fn watch(&mut self, path: PathBuf, source_idx: usize) -> Result<(), Error>;
    #[allow(dead_code)]
    fn unwatch(&mut self, path: &Path) -> Result<(), Error>;
}

// ---

/// Constructs the appropriate native backend for the current platform.
#[cfg(not(target_os = "macos"))]
pub fn new_native(tx: SyncSender<WakeSignal>, poll_interval: Duration) -> Result<Box<dyn Backend>, Error> {
    native::NativeBackend::new(tx, poll_interval).map(|b| Box::new(b) as Box<dyn Backend>)
}

#[cfg(target_os = "macos")]
pub fn new_native(tx: SyncSender<WakeSignal>, _poll_interval: Duration) -> Result<Box<dyn Backend>, Error> {
    kqueue_macos::KqueueBackend::new(tx).map(|b| Box::new(b) as Box<dyn Backend>)
}

pub fn new_poll(tx: SyncSender<WakeSignal>, poll_interval: Duration) -> Result<Box<dyn Backend>, Error> {
    poll::PollBackend::new(tx, poll_interval).map(|b| Box::new(b) as Box<dyn Backend>)
}
