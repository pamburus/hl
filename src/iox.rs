// std imports
use std::io::{Read, Result};

#[cfg(unix)]
use std::{
    io::{Error, ErrorKind},
    os::unix::io::AsRawFd,
    time::Duration,
};

// ---

pub trait ReadFill {
    fn read_fill(&mut self, buf: &mut [u8]) -> Result<usize>;
}

impl<T: Read + ?Sized> ReadFill for T {
    fn read_fill(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut i = 0;
        while i < buf.len() {
            let n = self.read(&mut buf[i..])?;
            if n == 0 {
                break;
            }
            i += n;
        }
        Ok(i)
    }
}

// ---

/// A reader wrapper that periodically checks a cancellation condition.
/// On Unix, uses poll() to avoid blocking indefinitely on read().
/// This allows cooperative cancellation of stdin reads in follow mode.
#[cfg(unix)]
pub struct CancellableReader<R, F> {
    inner: R,
    is_cancelled: F,
    poll_timeout: Duration,
}

#[cfg(unix)]
impl<R, F> CancellableReader<R, F>
where
    R: Read + AsRawFd,
    F: Fn() -> bool,
{
    /// Creates a new cancellable reader.
    ///
    /// # Arguments
    /// * `inner` - The underlying reader (must implement AsRawFd)
    /// * `is_cancelled` - A function that returns true when cancellation is requested
    /// * `poll_timeout` - How often to check for cancellation when no data is available
    pub fn new(inner: R, is_cancelled: F, poll_timeout: Duration) -> Self {
        Self {
            inner,
            is_cancelled,
            poll_timeout,
        }
    }

    /// Polls the file descriptor with a timeout.
    /// Returns:
    /// - Ok(true) if data is available for reading
    /// - Ok(false) if timeout occurred (no data available)
    /// - Err if poll failed or EOF/error detected
    fn poll_read(&self) -> Result<bool> {
        use libc::{POLLERR, POLLHUP, POLLIN, POLLNVAL, poll, pollfd};

        let fd = self.inner.as_raw_fd();
        let timeout_ms = self.poll_timeout.as_millis() as i32;

        let mut pfd = pollfd {
            fd,
            events: POLLIN,
            revents: 0,
        };

        let ret = unsafe { poll(&mut pfd, 1, timeout_ms) };

        if ret == -1 {
            return Err(Error::last_os_error());
        }

        if ret == 0 {
            // Timeout - no data available
            return Ok(false);
        }

        // Check for errors or hangup
        if (pfd.revents & POLLNVAL) != 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid file descriptor"));
        }

        if (pfd.revents & (POLLERR | POLLHUP)) != 0 && (pfd.revents & POLLIN) == 0 {
            // Error or hangup with no data available - treat as EOF
            return Err(Error::new(ErrorKind::UnexpectedEof, "pipe closed"));
        }

        // Data is available (POLLIN set, or POLLHUP with data still readable)
        Ok(true)
    }
}

#[cfg(unix)]
impl<R, F> Read for CancellableReader<R, F>
where
    R: Read + AsRawFd,
    F: Fn() -> bool,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        loop {
            // Check cancellation before polling
            if (self.is_cancelled)() {
                return Err(Error::new(ErrorKind::Interrupted, "cancelled"));
            }

            match self.poll_read() {
                Ok(true) => {
                    // Data available, do the actual read
                    return self.inner.read(buf);
                }
                Ok(false) => {
                    // Timeout, loop back and check cancellation
                    continue;
                }
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                    // EOF detected via poll
                    return Ok(0);
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {
                    // Interrupted by signal, retry
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}

#[cfg(unix)]
impl<R, F> AsRawFd for CancellableReader<R, F>
where
    R: AsRawFd,
{
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.inner.as_raw_fd()
    }
}

// ---

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    use std::{
        io::Write,
        os::unix::io::{FromRawFd, OwnedFd},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::Duration,
    };

    fn create_pipe() -> Result<(std::fs::File, std::fs::File)> {
        let mut fds = [0i32; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
            return Err(Error::last_os_error());
        }
        let read_fd = unsafe { OwnedFd::from_raw_fd(fds[0]) };
        let write_fd = unsafe { OwnedFd::from_raw_fd(fds[1]) };
        Ok((std::fs::File::from(read_fd), std::fs::File::from(write_fd)))
    }

    #[test]
    fn test_cancellable_reader_normal_read() {
        let (read_end, mut write_end) = create_pipe().unwrap();

        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = Arc::clone(&cancelled);

        let mut reader = CancellableReader::new(
            read_end,
            move || cancelled_clone.load(Ordering::SeqCst),
            Duration::from_millis(50),
        );

        // Write some data
        write_end.write_all(b"hello").unwrap();

        // Read should succeed
        let mut buf = [0u8; 10];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..n], b"hello");
    }

    #[test]
    fn test_cancellable_reader_cancellation() {
        let (read_end, _write_end) = create_pipe().unwrap();

        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = Arc::clone(&cancelled);

        let mut reader = CancellableReader::new(
            read_end,
            move || cancelled_clone.load(Ordering::SeqCst),
            Duration::from_millis(50),
        );

        // Cancel after a short delay in another thread
        let cancelled_setter = Arc::clone(&cancelled);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            cancelled_setter.store(true, Ordering::SeqCst);
        });

        // Read should eventually return Interrupted error
        let mut buf = [0u8; 10];
        let result = reader.read(&mut buf);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), ErrorKind::Interrupted);
    }

    #[test]
    fn test_cancellable_reader_eof_on_close() {
        let (read_end, write_end) = create_pipe().unwrap();

        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = Arc::clone(&cancelled);

        let mut reader = CancellableReader::new(
            read_end,
            move || cancelled_clone.load(Ordering::SeqCst),
            Duration::from_millis(50),
        );

        // Close write end after a short delay
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            drop(write_end);
        });

        // Read should return 0 (EOF)
        let mut buf = [0u8; 10];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 0);
    }
}
