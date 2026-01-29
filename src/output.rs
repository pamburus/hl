use std::env;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::{
    io::{IsTerminal, stdin},
    os::unix::io::AsRawFd,
    os::unix::process::ExitStatusExt,
};

use crate::error::*;

const MONITOR_CHECK_INTERVAL: Duration = Duration::from_millis(50);

pub type OutputStream = Box<dyn Write + Send + Sync>;

/// A monitor that can check if the output target has closed.
/// This is used to detect when a pager or downstream process exits.
pub trait OutputMonitor: Send + Sync {
    /// Returns true if the output has closed (e.g., pager exited).
    fn is_closed(&self) -> bool;
}

/// A no-op monitor that always reports the output as open.
pub struct NoOpMonitor;

impl OutputMonitor for NoOpMonitor {
    fn is_closed(&self) -> bool {
        false
    }
}

/// Monitor for stdout that detects when the read end of the pipe closes.
/// On macOS, this uses kqueue to detect EOF on the write end.
/// On Linux, this uses poll() to detect POLLHUP/POLLERR on the write end.
/// On other platforms, this is a no-op (returns false).
pub struct StdoutMonitor {
    closed: Arc<AtomicBool>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    _handle: Option<thread::JoinHandle<()>>,
}

impl StdoutMonitor {
    /// Creates a new stdout monitor.
    /// On macOS, this spawns a background thread that uses kqueue to detect pipe closure.
    #[cfg(target_os = "macos")]
    pub fn new() -> Self {
        use std::io::stdout;

        let closed = Arc::new(AtomicBool::new(false));
        let closed_clone = Arc::clone(&closed);

        // Get the raw fd for stdout
        let stdout_fd = stdout().as_raw_fd();

        let handle = thread::spawn(move || {
            Self::monitor_with_kqueue(stdout_fd, closed_clone);
        });

        Self {
            closed,
            _handle: Some(handle),
        }
    }

    #[cfg(target_os = "macos")]
    fn monitor_with_kqueue(fd: std::os::unix::io::RawFd, closed: Arc<AtomicBool>) {
        use libc::{EV_ADD, EV_CLEAR, EV_ENABLE, EV_EOF, EVFILT_WRITE, close, kevent, kqueue, timespec};

        unsafe {
            let kq = kqueue();
            if kq == -1 {
                log::debug!("stdout monitor: kqueue() failed");
                return;
            }

            // Register for write events with EV_CLEAR to get EOF notification
            let mut ev: libc::kevent = std::mem::zeroed();
            ev.ident = fd as usize;
            ev.filter = EVFILT_WRITE;
            ev.flags = EV_ADD | EV_ENABLE | EV_CLEAR;

            if kevent(kq, &ev, 1, std::ptr::null_mut(), 0, std::ptr::null()) == -1 {
                log::debug!("stdout monitor: kevent register failed");
                close(kq);
                return;
            }

            log::debug!("stdout monitor: started monitoring fd {}", fd);

            loop {
                if closed.load(Ordering::SeqCst) {
                    log::debug!("stdout monitor: already closed, exiting");
                    break;
                }

                let timeout = timespec {
                    tv_sec: 0,
                    tv_nsec: MONITOR_CHECK_INTERVAL.as_nanos() as i64,
                };

                let mut event: libc::kevent = std::mem::zeroed();
                let n = kevent(kq, std::ptr::null(), 0, &mut event, 1, &timeout);

                if n == -1 {
                    log::debug!("stdout monitor: kevent wait failed");
                    break;
                }

                if n > 0 && (event.flags & EV_EOF) != 0 {
                    log::debug!("stdout monitor: EOF detected on stdout");
                    closed.store(true, Ordering::SeqCst);
                    break;
                }
            }

            close(kq);
            log::debug!("stdout monitor: stopped");
        }
    }

    /// Creates a new stdout monitor (Linux version using poll).
    #[cfg(target_os = "linux")]
    pub fn new() -> Self {
        use std::io::stdout;

        let closed = Arc::new(AtomicBool::new(false));
        let closed_clone = Arc::clone(&closed);

        // Get the raw fd for stdout
        let stdout_fd = stdout().as_raw_fd();

        let handle = thread::spawn(move || {
            Self::monitor_with_poll(stdout_fd, closed_clone);
        });

        Self {
            closed,
            _handle: Some(handle),
        }
    }

    #[cfg(target_os = "linux")]
    fn monitor_with_poll(fd: std::os::unix::io::RawFd, closed: Arc<AtomicBool>) {
        use libc::{POLLERR, POLLHUP, POLLNVAL, poll, pollfd};

        let timeout_ms = MONITOR_CHECK_INTERVAL.as_millis() as i32;

        let mut pfd = pollfd {
            fd,
            events: 0, // We only care about POLLHUP/POLLERR which are always reported
            revents: 0,
        };

        log::debug!("stdout monitor: started monitoring fd {} (linux)", fd);

        loop {
            if closed.load(Ordering::SeqCst) {
                log::debug!("stdout monitor: already closed, exiting");
                break;
            }

            let ret = unsafe { poll(&mut pfd, 1, timeout_ms) };

            if ret == -1 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                log::debug!("stdout monitor: poll() failed: {}", err);
                break;
            }

            if ret > 0 {
                if (pfd.revents & POLLNVAL) != 0 {
                    log::debug!("stdout monitor: POLLNVAL - invalid fd");
                    break;
                }

                if (pfd.revents & (POLLERR | POLLHUP)) != 0 {
                    log::debug!("stdout monitor: POLLHUP/POLLERR detected on stdout");
                    closed.store(true, Ordering::SeqCst);
                    break;
                }
            }

            // Reset revents for next iteration
            pfd.revents = 0;
        }

        log::debug!("stdout monitor: stopped");
    }

    /// Creates a new stdout monitor (non-macOS/non-Linux version - no-op).
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub fn new() -> Self {
        Self {
            closed: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for StdoutMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputMonitor for StdoutMonitor {
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

/// Monitor for a Pager that checks if the pager process has exited.
pub struct PagerMonitor {
    inner: Arc<Mutex<PagerState>>,
}

impl OutputMonitor for PagerMonitor {
    fn is_closed(&self) -> bool {
        let mut state = self.inner.lock().unwrap();
        if state.exited {
            return true;
        }
        match state.process.try_wait() {
            Ok(Some(_status)) => {
                log::debug!("pager monitor: process has exited");
                state.exited = true;
                true
            }
            Ok(None) => false,
            Err(e) => {
                log::debug!("pager monitor: try_wait error: {}", e);
                false
            }
        }
    }
}

struct PagerState {
    process: Child,
    exited: bool,
}

pub struct Pager {
    state: Arc<Mutex<PagerState>>,
}

impl Pager {
    pub fn new() -> Result<Self> {
        let mut pager = "less".to_owned();

        if let Ok(p) = env::var("HL_PAGER") {
            if !p.is_empty() {
                pager = p;
            }
        } else if let Ok(p) = env::var("PAGER") {
            if !p.is_empty() {
                pager = p;
            }
        };

        let pager = shellwords::split(&pager).unwrap_or(vec![pager]);
        let (pager, args) = match pager.split_first() {
            Some((pager, args)) => (pager, args),
            None => (&pager[0], &pager[0..0]),
        };
        let pager = PathBuf::from(pager);
        let mut command = Command::new(&pager);
        for arg in args {
            command.arg(arg);
        }
        if pager.file_stem() == Some(&OsString::from("less")) {
            command.arg("-R");
            command.env("LESSCHARSET", "UTF-8");
        }

        let process = command.stdin(Stdio::piped()).spawn()?;

        Ok(Self {
            state: Arc::new(Mutex::new(PagerState { process, exited: false })),
        })
    }

    /// Creates a monitor that can be used to check if the pager has exited.
    /// The monitor can be safely shared across threads.
    pub fn monitor(&self) -> PagerMonitor {
        PagerMonitor {
            inner: Arc::clone(&self.state),
        }
    }

    #[cfg(unix)]
    fn recover(status: ExitStatus) {
        if let Some(signal) = status.signal() {
            if signal == 9 {
                eprintln!("\x1bm\nhl: pager killed");
                if stdin().is_terminal() {
                    Command::new("stty").arg("echo").status().ok();
                }
            }
        }
    }

    #[cfg(not(unix))]
    #[allow(unused_variables)]
    fn recover(status: ExitStatus) {}
}

impl Drop for Pager {
    fn drop(&mut self) {
        log::debug!("pager: drop called, waiting for process to exit");
        let mut state = self.state.lock().unwrap();
        if let Ok(status) = state.process.wait() {
            log::debug!("pager: process exited with status: {:?}", status);
            state.exited = true;
            Self::recover(status);
        } else {
            log::debug!("pager: wait() failed");
        }
        log::debug!("pager: drop finished");
    }
}

impl Write for Pager {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut state = self.state.lock().unwrap();
        let result = state.process.stdin.as_mut().unwrap().write(buf);
        if let Err(ref e) = result {
            log::debug!("pager write error: {} (kind: {:?})", e, e.kind());
        }
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut state = self.state.lock().unwrap();
        let result = state.process.stdin.as_mut().unwrap().flush();
        if let Err(ref e) = result {
            log::debug!("pager flush error: {} (kind: {:?})", e, e.kind());
        }
        result
    }
}
