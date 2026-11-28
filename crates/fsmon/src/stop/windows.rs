use std::{
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{Arc, Condvar, Mutex, PoisonError, Weak},
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_NOT_FOUND, HANDLE},
    System::{
        IO::CancelSynchronousIo,
        Threading::{GetCurrentProcess, GetCurrentThread},
    },
};

use super::Stop;
use crate::wait::Wake;

// ---

/// Wakes the waits and cancels the reads of the streams once stopped.
#[derive(Default)]
pub struct Waker {
    signals: Signals,
    reads: Reads,
}

impl Waker {
    pub fn new() -> io::Result<Self> {
        Ok(Self::default())
    }

    pub fn wake(&self) {
        self.signals.wake_all();
        self.reads.cancel_all();
    }

    pub fn signal(&self) -> Arc<Signal> {
        self.signals.register()
    }

    pub fn reads(&self) -> &Reads {
        &self.reads
    }
}

pub fn sleep(stop: &Stop, timeout: Duration) -> io::Result<Wake> {
    Ok(stop.signal().wait(Some(timeout), stop))
}

pub fn read(stop: &Stop, read: impl FnOnce() -> io::Result<usize>) -> io::Result<usize> {
    let _read = stop.waker.reads().register();
    // Checked once the read is registered, as a stop that comes after that cancels it.
    if stop.is_stopped() {
        return Ok(0);
    }
    match read() {
        Err(_) if stop.is_stopped() => Ok(0),
        result => result,
    }
}

// ---

/// Wakes a waiting stream, from a native watcher or a stop.
#[derive(Default)]
pub struct Signal {
    pending: Mutex<bool>,
    cond: Condvar,
}

impl Signal {
    pub fn wake(&self) {
        *self.pending.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.cond.notify_all();
    }

    /// Waits for a wake, the timeout, or a stop, whichever comes first.
    pub fn wait(&self, timeout: Option<Duration>, stop: &Stop) -> Wake {
        // A deadline too far to be represented is none.
        let deadline = timeout.and_then(|timeout| Instant::now().checked_add(timeout));
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            // The flag is set before the signals are woken, and a wake takes this lock, so it cannot be missed.
            if stop.is_stopped() {
                return Wake::Stopped;
            }
            if std::mem::take(&mut *pending) {
                return Wake::Ready;
            }
            pending = match deadline {
                None => self.cond.wait(pending).unwrap_or_else(PoisonError::into_inner),
                Some(deadline) => {
                    let now = Instant::now();
                    if now >= deadline {
                        return Wake::Timeout;
                    }
                    self.cond
                        .wait_timeout(pending, deadline - now)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
            };
        }
    }
}

/// The signals to wake once stopped.
#[derive(Default)]
struct Signals(Mutex<Vec<Weak<Signal>>>);

impl Signals {
    fn register(&self) -> Arc<Signal> {
        let signal = Arc::new(Signal::default());
        let mut signals = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        signals.retain(|signal| signal.strong_count() != 0);
        signals.push(Arc::downgrade(&signal));
        signal
    }

    fn wake_all(&self) {
        for signal in self.0.lock().unwrap_or_else(PoisonError::into_inner).iter() {
            if let Some(signal) = signal.upgrade() {
                signal.wake();
            }
        }
    }
}

// ---

/// How long a stop keeps trying to cancel a read that has not started yet.
const CANCEL_PATIENCE: Duration = Duration::from_secs(1);

thread_local! {
    /// A handle of the current thread that other threads can use, unlike the one `GetCurrentThread` returns.
    static THREAD: Option<OwnedHandle> = current_thread().ok();
}

fn current_thread() -> io::Result<OwnedHandle> {
    let mut handle = std::ptr::null_mut();
    // SAFETY: the pseudo handles of the current process and thread are always valid, and `handle` receives the
    // new handle.
    let ok = unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            GetCurrentThread(),
            GetCurrentProcess(),
            &mut handle,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the handle is new and owned by nothing else.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// The threads in a read that a stop cancels, by their handles.
#[derive(Default)]
pub struct Reads(Mutex<Vec<usize>>);

impl Reads {
    /// Registers the read of the current thread until the returned guard is dropped.
    fn register(&self) -> Option<Registered<'_>> {
        let handle = THREAD.with(|thread| thread.as_ref().map(|thread| thread.as_raw_handle() as usize))?;
        self.lock().push(handle);
        Some(Registered { reads: self, handle })
    }

    /// Cancels the registered reads.
    ///
    /// A read that has not started yet cannot be cancelled, so it is tried again until it starts or ends, for a
    /// while, as a read that cannot be cancelled would never end.
    fn cancel_all(&self) {
        let deadline = Instant::now() + CANCEL_PATIENCE;
        let mut cancelled = Vec::new();
        loop {
            let mut pending = false;
            for &handle in self.lock().iter() {
                if cancelled.contains(&handle) {
                    continue;
                }
                // SAFETY: the handle is open while it is registered, which the lock keeps it.
                if unsafe { CancelSynchronousIo(handle as HANDLE) } != 0 {
                    cancelled.push(handle);
                } else if io::Error::last_os_error().raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                    pending = true;
                }
            }
            if !pending || Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<usize>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Unregisters a read once dropped.
struct Registered<'a> {
    reads: &'a Reads,
    handle: usize,
}

impl Drop for Registered<'_> {
    fn drop(&mut self) {
        let mut reads = self.reads.lock();
        if let Some(i) = reads.iter().position(|&handle| handle == self.handle) {
            reads.swap_remove(i);
        }
    }
}
