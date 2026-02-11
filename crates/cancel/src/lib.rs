use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};

#[cfg(windows)]
use std::os::windows::io::OwnedHandle;

// ---

/// A cooperative, event-driven cancellation token.
///
/// On Unix, uses a pipe for signaling so that the read fd can be integrated
/// with `poll()` or `kqueue` for immediate wakeup.
/// On Windows, uses an event object.
pub struct CancellationToken {
    cancelled: AtomicBool,
    #[cfg(unix)]
    pipe: (OwnedFd, OwnedFd),
    #[cfg(windows)]
    event: OwnedHandle,
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
            unsafe {
                (
                    OwnedFd::from_raw_fd(fds[0]),
                    OwnedFd::from_raw_fd(fds[1]),
                )
            }
        };

        #[cfg(windows)]
        let event = {
            use windows_sys::Win32::System::Threading::CreateEventW;

            let handle = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
            if handle == 0 {
                return Err(io::Error::last_os_error());
            }
            unsafe { OwnedHandle::from_raw_handle(handle as std::os::windows::io::RawHandle) }
        };

        Ok(Self {
            cancelled: AtomicBool::new(false),
            #[cfg(unix)]
            pipe,
            #[cfg(windows)]
            event,
        })
    }

    /// Signals cancellation. This is thread-safe and triggers immediate wakeup
    /// of any thread waiting on the associated fd or event.
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

        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::System::Threading::SetEvent;

            unsafe {
                SetEvent(self.event.as_raw_handle() as isize);
            }
        }
    }

    /// Returns whether cancellation has been signaled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Wraps a reader so that reads can be interrupted by cancellation.
    ///
    /// On Unix, uses `poll()` to wait on both the reader's fd and the cancellation pipe,
    /// returning `ErrorKind::Interrupted` when cancelled.
    /// On other platforms, delegates to the inner reader directly.
    pub fn wrap_read<R>(&self, reader: R) -> CancellableReader<'_, R> {
        CancellableReader {
            inner: reader,
            token: self,
        }
    }

    /// Returns a guard that calls `cancel()` when dropped.
    pub fn drop_guard(&self) -> CancelGuard<'_> {
        CancelGuard(self)
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
pub struct CancelGuard<'a>(&'a CancellationToken);

impl Drop for CancelGuard<'_> {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

// ---

/// A reader wrapper that supports cancellation.
pub struct CancellableReader<'a, R> {
    inner: R,
    token: &'a CancellationToken,
}

#[cfg(unix)]
impl<R: AsRawFd + io::Read> io::Read for CancellableReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut pollfds = [
            libc::pollfd {
                fd: self.inner.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.token.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        loop {
            let ret = unsafe { libc::poll(pollfds.as_mut_ptr(), 2, -1) };
            if ret < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }

            if pollfds[1].revents & libc::POLLIN != 0 {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }

            if pollfds[0].revents & libc::POLLIN != 0 {
                let ret = unsafe {
                    libc::read(
                        self.inner.as_raw_fd(),
                        buf.as_mut_ptr() as *mut libc::c_void,
                        buf.len(),
                    )
                };
                if ret < 0 {
                    return Err(io::Error::last_os_error());
                }
                return Ok(ret as usize);
            }

            if pollfds[0].revents & (libc::POLLHUP | libc::POLLERR) != 0 {
                return Ok(0);
            }
        }
    }
}

#[cfg(not(unix))]
impl<R: io::Read> io::Read for CancellableReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
