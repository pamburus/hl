//! Ends every stream of a [`Follow`](crate::Follow) at once.

use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::wait::Wake;

// ---

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as imp;
#[cfg(windows)]
pub(crate) mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(target_os = "macos")]
pub(crate) use unix::set_flag;

// ---

/// Stops a [`Follow`](crate::Follow) from any thread.
///
/// Once stopped, every read of its streams returns the end of the stream at once, including a read that is blocked
/// waiting for data, and no input starts another stream. On Windows, a read of the console may not end, as it cannot
/// be cancelled reliably there.
#[derive(Clone)]
pub struct Stopper(pub(crate) Arc<Stop>);

impl Stopper {
    /// Stops the follow. Stopping it again does nothing.
    pub fn stop(&self) {
        self.0.stop();
    }
}

// ---

/// The state shared by a follow, its stopper and its streams.
pub(crate) struct Stop {
    stopped: AtomicBool,
    waker: imp::Waker,
}

impl Stop {
    pub fn new() -> io::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            stopped: AtomicBool::new(false),
            waker: imp::Waker::new()?,
        }))
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }

    pub fn stop(&self) {
        if !self.stopped.swap(true, Ordering::AcqRel) {
            self.waker.wake();
        }
    }

    /// Waits for the timeout unless stopped first.
    pub fn sleep(&self, timeout: Duration) -> io::Result<Wake> {
        imp::sleep(self, timeout)
    }

    /// Returns a descriptor that becomes readable once stopped, and stays readable.
    #[cfg(unix)]
    pub fn fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.waker.fd()
    }

    /// Returns a new signal that is woken once stopped.
    #[cfg(windows)]
    pub fn signal(&self) -> Arc<windows::Signal> {
        self.waker.signal()
    }

    /// Makes `read`, a read that blocks until there is data, end once stopped, when it returns the end of the stream.
    ///
    /// A read of the console may not end, as the standard library reads it again when its read is cancelled.
    #[cfg(windows)]
    pub fn read(&self, read: impl FnOnce() -> io::Result<usize>) -> io::Result<usize> {
        imp::read(self, read)
    }
}
