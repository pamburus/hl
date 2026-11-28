//! Helpers shared by the integration tests.

use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
    sync::mpsc::{Receiver, RecvTimeoutError, channel},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use fsmon::{FallbackPolicy, Follow, FollowOptions, Input, Stopper};

/// Appends `data` to the file at `path` in one write, creating the file if it does not exist.
pub fn append(path: &Path, data: impl AsRef<[u8]>) {
    let mut file = OpenOptions::new().create(true).append(true).open(path).unwrap();
    file.write_all(data.as_ref()).unwrap();
}

/// How long a test waits for something that is expected to happen.
pub const PATIENCE: Duration = Duration::from_secs(5);

/// How a followed path is waited for.
#[derive(Clone, Copy, Debug)]
pub enum Mode {
    /// With native notifications, as a path on a local file system is.
    Native,
    /// By polling.
    Polled,
}

/// Creates a follow with `options`, which waits for changes as `mode` tells.
pub fn follow(mode: Mode, mut options: FollowOptions) -> Follow {
    if let Mode::Polled = mode {
        options.fallback_policy = FallbackPolicy::Poll;
    }
    Follow::new(options).unwrap()
}

/// What the reader of an input saw.
#[derive(Debug, PartialEq)]
pub enum Event {
    Data(Vec<u8>),
    /// A stream ended.
    End,
    /// The input ended.
    Done,
    Error(String),
}

/// Reads all streams of an input in a thread of its own, and stops the follow once dropped.
pub struct Reader {
    stopper: Stopper,
    events: Receiver<Event>,
    thread: Option<JoinHandle<()>>,
    /// Data received but not yet expected.
    pending: Vec<u8>,
}

impl Reader {
    pub fn spawn(follow: &Follow, mut input: Input) -> Self {
        let (tx, events) = channel();
        let thread = thread::spawn(move || {
            loop {
                let mut stream = match input.next_stream() {
                    Ok(Some(stream)) => stream,
                    Ok(None) => break,
                    Err(err) => {
                        let _ = tx.send(Event::Error(err.to_string()));
                        return;
                    }
                };
                let mut buf = [0; 4096];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let _ = tx.send(Event::Data(buf[..n].to_vec()));
                        }
                        Err(err) => {
                            let _ = tx.send(Event::Error(err.to_string()));
                            return;
                        }
                    }
                }
                // Closed before the end is reported, so that the test may act on the file as no longer open.
                drop(stream);
                let _ = tx.send(Event::End);
            }
            let _ = tx.send(Event::Done);
        });
        Self {
            stopper: follow.stopper(),
            events,
            thread: Some(thread),
            pending: Vec::new(),
        }
    }

    /// Returns the thread that reads the input.
    #[cfg(unix)]
    pub fn thread(&self) -> &JoinHandle<()> {
        self.thread.as_ref().unwrap()
    }

    fn next(&self, timeout: Duration) -> Option<Event> {
        match self.events.recv_timeout(timeout) {
            Ok(event) => Some(event),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => panic!("the reader is gone"),
        }
    }

    /// Expects the data to come next, possibly in several parts.
    #[track_caller]
    pub fn expect_data(&mut self, expected: &str) {
        let deadline = Instant::now() + PATIENCE;
        while self.pending.len() < expected.len() {
            match self.next(deadline.saturating_duration_since(Instant::now())) {
                Some(Event::Data(data)) => self.pending.extend(data),
                other => panic!(
                    "expected {expected:?}, got {:?} then {other:?}",
                    String::from_utf8_lossy(&self.pending)
                ),
            }
        }
        let rest = self.pending.split_off(expected.len());
        assert_eq!(
            String::from_utf8_lossy(&std::mem::replace(&mut self.pending, rest)),
            expected
        );
    }

    /// Expects the event to come next, with no data before it.
    #[track_caller]
    pub fn expect(&mut self, expected: Event, timeout: Duration) {
        assert!(
            self.pending.is_empty(),
            "unexpected data {:?}",
            String::from_utf8_lossy(&self.pending)
        );
        assert_eq!(self.next(timeout), Some(expected));
    }

    /// Expects nothing to come for a while.
    #[track_caller]
    pub fn expect_quiet(&self, timeout: Duration) {
        assert!(self.pending.is_empty());
        assert_eq!(self.next(timeout), None);
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.stopper.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Makes a FIFO at `path`.
#[cfg(unix)]
pub fn mkfifo(path: &Path) {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let c = CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: `c` is a NUL-terminated string.
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
}

/// Makes a pipe that has no name, as one given by process substitution, returning its ends.
#[cfg(unix)]
pub fn pipe() -> (std::fs::File, std::fs::File) {
    use std::os::fd::FromRawFd;
    let mut fds = [0; 2];
    // SAFETY: `fds` has room for both ends of the pipe.
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    // SAFETY: both ends are open and owned by nothing else.
    unsafe { (std::fs::File::from_raw_fd(fds[0]), std::fs::File::from_raw_fd(fds[1])) }
}

/// Returns the path that opens `file` again, as the shell gives for process substitution.
#[cfg(unix)]
pub fn path_of(file: &std::fs::File) -> std::path::PathBuf {
    use std::os::fd::AsRawFd;
    std::path::PathBuf::from(format!("/dev/fd/{}", file.as_raw_fd()))
}

/// Interrupts a call that blocks `thread`, as a signal does, so that the call fails with `EINTR`.
///
/// The signal sent has a handler that does nothing and does not ask for the calls it interrupts to be restarted. A call
/// that does not block at the time is not interrupted, so a test sends it again until it sees the effect.
#[cfg(unix)]
pub fn interrupt<T>(thread: &JoinHandle<T>) {
    use std::{os::unix::thread::JoinHandleExt, sync::Once};

    extern "C" fn ignore(_: libc::c_int) {}

    static HANDLER: Once = Once::new();
    HANDLER.call_once(|| {
        // SAFETY: an all-zero `sigaction` is a valid value of a plain C struct, and the handler does nothing.
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = ignore as extern "C" fn(libc::c_int) as libc::sighandler_t;
            libc::sigemptyset(&mut action.sa_mask);
            assert_eq!(libc::sigaction(libc::SIGUSR1, &action, std::ptr::null_mut()), 0);
        }
    });
    // SAFETY: the thread is not joined yet, so its handle is not reused by another thread.
    let result = unsafe { libc::pthread_kill(thread.as_pthread_t(), libc::SIGUSR1) };
    // A thread that has just ended has nothing left to interrupt, which some platforms report as an error.
    assert!(result == 0 || result == libc::ESRCH, "pthread_kill failed: {result}");
}

/// Interrupts the calls that block `thread` for a while, as signals that come one after another do.
#[cfg(unix)]
pub fn interrupt_for_a_while<T>(thread: &JoinHandle<T>) {
    for _ in 0..20 {
        interrupt(thread);
        thread::sleep(Duration::from_millis(5));
    }
}

/// Returns the debug messages logged by the tests so far, once it has started to collect them with its first call.
///
/// The messages of all tests are collected together, as a process has one logger, so a test looks only for those about
/// its own paths.
#[cfg(target_os = "linux")]
pub fn debug_log() -> Vec<String> {
    use std::sync::{Mutex, OnceLock};

    struct Collector(Mutex<Vec<String>>);

    impl log::Log for Collector {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::Level::Debug
        }

        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                self.0.lock().unwrap().push(record.args().to_string());
            }
        }

        fn flush(&self) {}
    }

    static COLLECTOR: OnceLock<&'static Collector> = OnceLock::new();
    let collector = COLLECTOR.get_or_init(|| {
        let collector = Box::leak(Box::new(Collector(Mutex::new(Vec::new()))));
        log::set_logger(collector).unwrap();
        log::set_max_level(log::LevelFilter::Debug);
        collector
    });
    collector.0.lock().unwrap().clone()
}
