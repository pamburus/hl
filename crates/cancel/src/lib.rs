use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};

// ---

/// A cooperative, event-driven cancellation token.
///
/// On Unix, uses a pipe for signaling so that the read fd can be integrated
/// with `poll()` or `kqueue` for immediate wakeup.
pub struct CancellationToken {
    cancelled: AtomicBool,
    #[cfg(unix)]
    pipe: (OwnedFd, OwnedFd),
}

impl CancellationToken {
    /// Creates a new cancellation token.
    pub fn new() -> io::Result<Self> {
        #[cfg(unix)]
        let pipe = {
            let mut fds = [0i32; 2];
            if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
                return Err(io::Error::last_os_error());
            }
            unsafe { libc::fcntl(fds[0], libc::F_SETFL, libc::O_NONBLOCK) };
            unsafe { libc::fcntl(fds[1], libc::F_SETFL, libc::O_NONBLOCK) };
            unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) }
        };

        Ok(Self {
            cancelled: AtomicBool::new(false),
            #[cfg(unix)]
            pipe,
        })
    }

    /// Signals cancellation. Thread-safe and triggers immediate wakeup
    /// of any thread waiting on the associated fd.
    pub fn cancel(&self) {
        if self.cancelled.swap(true, Ordering::SeqCst) {
            return;
        }

        #[cfg(unix)]
        {
            unsafe {
                libc::write(self.pipe.1.as_raw_fd(), [1u8].as_ptr() as *const libc::c_void, 1);
            }
        }
    }

    /// Returns whether cancellation has been signaled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Returns a guard that calls `cancel()` when dropped.
    pub fn drop_guard(self: &Arc<Self>) -> CancelGuard {
        CancelGuard(self.clone())
    }
}

#[cfg(unix)]
impl AsRawFd for CancellationToken {
    /// Returns the read end of the cancellation pipe.
    /// This fd becomes readable when `cancel()` is called, and can be used
    /// with `poll()` or `kqueue` for event-driven wakeup.
    fn as_raw_fd(&self) -> RawFd {
        self.pipe.0.as_raw_fd()
    }
}

// ---

/// A guard that cancels the associated token when dropped.
pub struct CancelGuard(Arc<CancellationToken>);

impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

// ---

/// Wraps a value, drops it, then signals a `CancellationToken`.
pub struct DropNotifier<T> {
    inner: Option<T>,
    token: Arc<CancellationToken>,
}

impl<T> DropNotifier<T> {
    /// Creates a new `DropNotifier` wrapping `value`.
    ///
    /// When this `DropNotifier` is dropped, it will first drop `value`,
    /// then signal cancellation on `token`.
    pub fn new(value: T, token: Arc<CancellationToken>) -> Self {
        Self {
            inner: Some(value),
            token,
        }
    }
}

impl<T> Drop for DropNotifier<T> {
    fn drop(&mut self) {
        self.inner.take(); // drops the inner value
        self.token.cancel();
    }
}

// ---

/// Drops a value in a background thread, joining on its own drop.
///
/// This is useful when dropping a value may block (e.g. `Child::wait()`),
/// and you want the drop to happen asynchronously while still ensuring
/// it completes before the owning scope exits.
pub struct AsyncDrop {
    handle: Option<JoinHandle<()>>,
}

impl AsyncDrop {
    /// Spawns a background thread that takes ownership of `value` and drops it.
    pub fn new<T: Send + 'static>(value: T) -> Self {
        let handle = thread::spawn(move || {
            drop(value);
        });
        Self {
            handle: Some(handle),
        }
    }
}

impl Drop for AsyncDrop {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().ok();
        }
    }
}
