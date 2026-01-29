// std imports
use std::path::PathBuf;

// local imports
use crate::error::Result;
use crate::output::PipeCloseSignal;

// ---

pub type Event = notify::Event;
pub type EventKind = notify::EventKind;

// ---

/// A cancellation token that can be used to signal fsmon to stop watching.
///
/// This uses a pipe-based signaling mechanism for immediate, event-based wakeup.
/// The implementation is platform-specific but provides a unified interface.
pub struct Cancellation {
    inner: imp::Cancellation,
    /// Optional signal to monitor for pipe closure (e.g., pager stdin).
    /// When this signals, it triggers cancellation.
    close_signal: Option<PipeCloseSignal>,
}

impl Cancellation {
    /// Creates a new cancellation token.
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            inner: imp::Cancellation::new()?,
            close_signal: None,
        })
    }

    /// Configures the cancellation to also trigger when the given pipe close signal fires.
    ///
    /// This is useful for detecting when a pager process exits, allowing follow mode
    /// to terminate immediately.
    pub fn with_close_signal(mut self, signal: PipeCloseSignal) -> Self {
        self.close_signal = Some(signal);
        self
    }

    /// Signals cancellation. This is thread-safe and triggers immediate wakeup.
    pub fn cancel(&self) {
        log::debug!("fsmon::Cancellation::cancel() called");
        self.inner.cancel()
    }
}

// ---

#[allow(dead_code)]
pub fn run<H>(paths: Vec<PathBuf>, handle: H) -> Result<()>
where
    H: FnMut(Event) -> Result<()>,
{
    run_with_cancellation(paths, handle, None)
}

pub fn run_with_cancellation<H>(
    mut paths: Vec<PathBuf>,
    mut handle: H,
    cancellation: Option<&Cancellation>,
) -> Result<()>
where
    H: FnMut(Event) -> Result<()>,
{
    log::debug!("fsmon::run: starting with {} paths", paths.len());
    if paths.is_empty() {
        log::debug!("fsmon::run: no paths, returning early");
        return Ok(());
    }

    paths.retain(|path| path.metadata().is_ok_and(|metadata| metadata.file_type().is_file()));

    for i in 0..paths.len() {
        if let Ok(canonical_path) = paths[i].canonicalize() {
            match paths[i].symlink_metadata() {
                Ok(metadata) if metadata.file_type().is_symlink() => paths.push(canonical_path),
                _ => paths[i] = canonical_path,
            }
        }
    }

    paths.sort_unstable();
    paths.dedup();

    let mut watch = paths
        .iter()
        .map(|path| {
            let mut path = path.clone();
            path.pop();
            path
        })
        .collect::<Vec<PathBuf>>();
    watch.extend_from_slice(&paths);
    watch.sort_unstable();
    watch.dedup();

    log::debug!("fsmon::run: entering imp::run with {} watch paths", watch.len());

    #[cfg(unix)]
    let trigger_fd = cancellation.and_then(|c| c.close_signal.as_ref().map(|s| s.fd));
    #[cfg(not(unix))]
    let trigger_fd: Option<()> = None;

    let close_signal = cancellation.and_then(|c| c.close_signal.as_ref());

    imp::run(
        watch,
        |event| {
            log::debug!("fsmon::run: received event: {:?}", event.kind);
            if event.paths.iter().any(|path| paths.binary_search(path).is_ok()) {
                handle(event)
            } else {
                Ok(())
            }
        },
        cancellation.map(|c| &c.inner),
        trigger_fd,
        close_signal,
    )
}

// Platform-specific cancellation implementations
// Only used on non-macOS platforms; macOS has its own implementation in imp
#[cfg(all(unix, not(target_os = "macos")))]
mod cancellation {
    use std::io::{self, Read, Write};
    use std::os::unix::net::UnixStream;

    pub struct Cancellation {
        reader: UnixStream,
        writer: UnixStream,
    }

    impl Cancellation {
        pub fn new() -> io::Result<Self> {
            let (reader, writer) = UnixStream::pair()?;
            reader.set_nonblocking(true)?;
            writer.set_nonblocking(true)?;
            Ok(Self { reader, writer })
        }

        pub fn cancel(&self) {
            // Write a single byte to wake up the poll. Ignore errors (e.g., if already cancelled).
            let _ = (&self.writer).write(&[0u8]);
        }

        pub fn is_cancelled(&self) -> bool {
            let mut buf = [0u8; 1];
            matches!((&self.reader).read(&mut buf), Ok(1..))
        }
    }
}

#[cfg(windows)]
mod cancellation {
    use std::io;
    use std::sync::atomic::{AtomicBool, Ordering};

    pub struct Cancellation {
        cancelled: AtomicBool,
    }

    impl Cancellation {
        pub fn new() -> io::Result<Self> {
            Ok(Self {
                cancelled: AtomicBool::new(false),
            })
        }

        pub fn cancel(&self) {
            self.cancelled.store(true, Ordering::SeqCst);
        }

        pub fn is_cancelled(&self) -> bool {
            self.cancelled.load(Ordering::SeqCst)
        }
    }
}

// Non-macOS Unix implementation using notify's RecommendedWatcher with poll()
#[cfg(all(not(target_os = "macos"), unix))]
mod imp {
    use std::collections::VecDeque;
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixStream;
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    use super::*;
    use crate::error::Error;

    pub use super::cancellation::Cancellation;

    const FALLBACK_POLLING_INTERVAL: Duration = Duration::from_secs(1);

    type TriggerFd = std::os::unix::io::RawFd;

    pub fn run<H>(
        paths: Vec<PathBuf>,
        mut handle: H,
        cancellation: Option<&Cancellation>,
        trigger_fd: Option<TriggerFd>,
        _close_signal: Option<&PipeCloseSignal>,
    ) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        // If we have both cancellation and a trigger fd, use fully event-based poll() loop
        if cancellation.is_some() && trigger_fd.is_some() {
            let trigger_fd = trigger_fd.unwrap();
            let cancellation = cancellation.unwrap();

            // Create a socket pair for notify event signaling
            let (notify_reader, notify_writer) = UnixStream::pair()?;
            notify_reader.set_nonblocking(true)?;

            let notify_read_fd = notify_reader.as_raw_fd();

            // Shared queue for events from the notify thread
            let event_queue: Arc<Mutex<VecDeque<notify::Result<Event>>>> =
                Arc::new(Mutex::new(VecDeque::new()));
            let event_queue_writer = Arc::clone(&event_queue);

            // Create the watcher with a channel
            let (tx, rx) = mpsc::channel();
            let mut watcher =
                RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

            for path in &paths {
                watcher.watch(path, RecursiveMode::NonRecursive)?;
            }

            // Spawn thread to receive notify events and signal the main loop
            let _notify_thread = thread::spawn(move || {
                for event in rx {
                    // Store the event in the shared queue
                    if let Ok(mut queue) = event_queue_writer.lock() {
                        queue.push_back(event);
                    }
                    // Signal that an event is available (write a byte to wake up poll)
                    let _ = (&notify_writer).write(&[1u8]);
                }
            });

            // Main event loop using poll() with infinite timeout
            loop {
                // Build the poll fd array
                let mut pollfds = [
                    libc::pollfd {
                        fd: trigger_fd,
                        events: 0, // We only care about POLLHUP/POLLERR
                        revents: 0,
                    },
                    libc::pollfd {
                        fd: notify_read_fd,
                        events: libc::POLLIN as i16,
                        revents: 0,
                    },
                ];

                // Poll with infinite timeout (-1) - fully event-based, no busy waiting
                let poll_result = unsafe { libc::poll(pollfds.as_mut_ptr(), 2, -1) };

                if poll_result < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err.into());
                }

                // Check for cancellation
                if cancellation.is_cancelled() {
                    log::debug!("fsmon::imp::run (linux): cancellation detected");
                    return Ok(());
                }

                // Check for pipe closure (POLLHUP or POLLERR on trigger_fd)
                if (pollfds[0].revents & (libc::POLLHUP | libc::POLLERR) as i16) != 0 {
                    log::debug!("fsmon::imp::run (linux): pipe closed (POLLHUP/POLLERR), exiting");
                    return Ok(());
                }

                // Check for notify events
                if (pollfds[1].revents & libc::POLLIN as i16) != 0 {
                    // Drain the signaling bytes
                    let mut buf = [0u8; 64];
                    let _ = std::io::Read::read(&mut &notify_reader, &mut buf);

                    // Process all queued events
                    loop {
                        let event = {
                            let mut queue = event_queue.lock().unwrap();
                            queue.pop_front()
                        };
                        match event {
                            Some(Ok(event)) => handle(event)?,
                            Some(Err(err)) => return Err(err.into()),
                            None => break,
                        }
                    }
                }
            }
        } else if let Some(cancellation) = cancellation {
            // Cancellation without trigger fd - need to use a signaling mechanism
            let (cancel_reader, cancel_writer) = UnixStream::pair()?;
            cancel_reader.set_nonblocking(true)?;

            let cancel_read_fd = cancel_reader.as_raw_fd();

            // Shared queue for events
            let event_queue: Arc<Mutex<VecDeque<notify::Result<Event>>>> =
                Arc::new(Mutex::new(VecDeque::new()));
            let event_queue_writer = Arc::clone(&event_queue);

            let (tx, rx) = mpsc::channel();
            let mut watcher =
                RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

            for path in &paths {
                watcher.watch(path, RecursiveMode::NonRecursive)?;
            }

            // Spawn thread to receive notify events
            let _notify_thread = thread::spawn(move || {
                for event in rx {
                    if let Ok(mut queue) = event_queue_writer.lock() {
                        queue.push_back(event);
                    }
                    let _ = (&cancel_writer).write(&[1u8]);
                }
            });

            loop {
                let mut pollfds = [libc::pollfd {
                    fd: cancel_read_fd,
                    events: libc::POLLIN as i16,
                    revents: 0,
                }];

                let poll_result = unsafe { libc::poll(pollfds.as_mut_ptr(), 1, -1) };

                if poll_result < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err.into());
                }

                if cancellation.is_cancelled() {
                    log::debug!("fsmon::imp::run (linux): cancellation detected");
                    return Ok(());
                }

                if (pollfds[0].revents & libc::POLLIN as i16) != 0 {
                    let mut buf = [0u8; 64];
                    let _ = std::io::Read::read(&mut &cancel_reader, &mut buf);

                    loop {
                        let event = {
                            let mut queue = event_queue.lock().unwrap();
                            queue.pop_front()
                        };
                        match event {
                            Some(Ok(event)) => handle(event)?,
                            Some(Err(err)) => return Err(err.into()),
                            None => break,
                        }
                    }
                }
            }
        } else {
            // Original behavior without cancellation
            let (tx, rx) = mpsc::channel();
            let mut watcher =
                RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

            for path in &paths {
                watcher.watch(path, RecursiveMode::NonRecursive)?;
            }

            loop {
                match rx.recv() {
                    Ok(Ok(event)) => handle(event)?,
                    Ok(Err(err)) => return Err(err.into()),
                    Err(err) => return Err(Error::RecvTimeoutError { source: err.into() }),
                };
            }
        }
    }
}

// Windows implementation using notify's RecommendedWatcher
#[cfg(windows)]
mod imp {
    use std::collections::VecDeque;
    use std::sync::mpsc;
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;
    use std::time::Duration;

    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    use super::*;
    use crate::error::Error;

    pub use super::cancellation::Cancellation;

    const FALLBACK_POLLING_INTERVAL: Duration = Duration::from_secs(1);

    type TriggerFd = ();

    pub fn run<H>(
        paths: Vec<PathBuf>,
        mut handle: H,
        cancellation: Option<&Cancellation>,
        _trigger_fd: Option<TriggerFd>,
        close_signal: Option<&PipeCloseSignal>,
    ) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{
            CreateEventW, OpenProcess, SetEvent, WaitForMultipleObjects, INFINITE, PROCESS_SYNCHRONIZE,
        };

        if let Some(cancellation) = cancellation {
            // Create a Windows Event for signaling notify events
            let notify_event: HANDLE = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
            if notify_event.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }

            // Ensure we close the event handle when done
            struct EventGuard(HANDLE);
            impl Drop for EventGuard {
                fn drop(&mut self) {
                    unsafe { CloseHandle(self.0) };
                }
            }
            let _notify_event_guard = EventGuard(notify_event);

            // Shared queue for events
            let event_queue: Arc<Mutex<VecDeque<notify::Result<Event>>>> =
                Arc::new(Mutex::new(VecDeque::new()));
            let event_queue_writer = Arc::clone(&event_queue);

            // Shared flag for signaling termination
            let terminated = Arc::new((Mutex::new(false), Condvar::new()));
            let terminated_writer = Arc::clone(&terminated);

            let (tx, rx) = mpsc::channel();
            let mut watcher =
                RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

            for path in &paths {
                watcher.watch(path, RecursiveMode::NonRecursive)?;
            }

            // Spawn thread to receive notify events and signal via Windows Event
            // We cast HANDLE to isize to make it Send (it's safe because we only use it for SetEvent)
            let notify_event_raw = notify_event as isize;
            let _notify_thread = thread::spawn(move || {
                for event in rx {
                    if let Ok(mut queue) = event_queue_writer.lock() {
                        queue.push_back(event);
                    }
                    // Signal that an event is available
                    unsafe { SetEvent(notify_event_raw as HANDLE) };
                }
                // Channel closed, signal termination
                let (lock, _cvar) = &*terminated_writer;
                if let Ok(mut term) = lock.lock() {
                    *term = true;
                }
                unsafe { SetEvent(notify_event_raw as HANDLE) };
            });

            // Open the process handle if we have a close signal
            let process_handle: HANDLE = if let Some(signal) = close_signal {
                unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, signal.process_id) }
            } else {
                std::ptr::null_mut()
            };

            struct ProcessHandleGuard(HANDLE);
            impl Drop for ProcessHandleGuard {
                fn drop(&mut self) {
                    if !self.0.is_null() {
                        unsafe { CloseHandle(self.0) };
                    }
                }
            }
            let _process_handle_guard = ProcessHandleGuard(process_handle);

            // Main event loop using WaitForMultipleObjects with INFINITE timeout
            loop {
                if cancellation.is_cancelled() {
                    log::debug!("fsmon::imp::run (windows): cancellation detected");
                    return Ok(());
                }

                // Build the handle array for WaitForMultipleObjects
                let (handles, handle_count): ([HANDLE; 2], u32) = if !process_handle.is_null() {
                    ([notify_event, process_handle], 2)
                } else {
                    ([notify_event, std::ptr::null_mut()], 1)
                };

                // Wait with INFINITE timeout - fully event-based
                let wait_result =
                    unsafe { WaitForMultipleObjects(handle_count, handles.as_ptr(), 0, INFINITE) };

                if wait_result == WAIT_OBJECT_0 {
                    // Notify event signaled - process events from the queue
                    loop {
                        let event = {
                            let mut queue = event_queue.lock().unwrap();
                            queue.pop_front()
                        };
                        match event {
                            Some(Ok(event)) => handle(event)?,
                            Some(Err(err)) => return Err(err.into()),
                            None => break,
                        }
                    }

                    // Check if the notify thread terminated
                    let (lock, _) = &*terminated;
                    if let Ok(term) = lock.lock() {
                        if *term {
                            return Err(Error::RecvTimeoutError {
                                source: mpsc::RecvTimeoutError::Disconnected,
                            });
                        }
                    }
                } else if wait_result == WAIT_OBJECT_0 + 1 && !process_handle.is_null() {
                    // Process handle signaled - pager has exited
                    log::debug!("fsmon::imp::run (windows): pager process exited");
                    return Ok(());
                } else {
                    // Error or timeout (shouldn't happen with INFINITE)
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() != Some(0) {
                        return Err(err.into());
                    }
                }
            }
        } else {
            // Original behavior without cancellation
            let (tx, rx) = mpsc::channel();
            let mut watcher =
                RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

            for path in &paths {
                watcher.watch(path, RecursiveMode::NonRecursive)?;
            }

            loop {
                match rx.recv() {
                    Ok(Ok(event)) => handle(event)?,
                    Ok(Err(err)) => return Err(err.into()),
                    Err(err) => return Err(Error::RecvTimeoutError { source: err.into() }),
                };
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::io::{self, Write};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::os::unix::net::UnixStream;
    use std::{collections::HashSet, time::Duration};

    use kqueue::{EventData, EventFilter, FilterFlag, Ident, Vnode, Watcher};
    use notify::event::{CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode};

    use super::*;

    /// Duplicates a file descriptor. The caller owns the new fd.
    fn dup_fd(fd: RawFd) -> io::Result<OwnedFd> {
        let new_fd = unsafe { libc::dup(fd) };
        if new_fd == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(unsafe { OwnedFd::from_raw_fd(new_fd) })
        }
    }

    // macOS needs its own Cancellation with as_raw_fd() for kqueue integration
    pub struct Cancellation {
        reader: UnixStream,
        writer: UnixStream,
    }

    impl Cancellation {
        pub fn new() -> io::Result<Self> {
            let (reader, writer) = UnixStream::pair()?;
            Ok(Self { reader, writer })
        }

        pub fn cancel(&self) {
            // Write a single byte to wake up kqueue. Ignore errors (e.g., if already cancelled).
            let _ = (&self.writer).write(&[0u8]);
        }

        pub(super) fn as_raw_fd(&self) -> RawFd {
            self.reader.as_raw_fd()
        }
    }

    pub fn run<H>(
        paths: Vec<PathBuf>,
        mut handle: H,
        cancellation: Option<&Cancellation>,
        trigger_fd: Option<RawFd>,
        _close_signal: Option<&PipeCloseSignal>,
    ) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        // Note: _close_signal is not used on macOS since we use the fd-based trigger_fd mechanism
        log::debug!("fsmon::imp::run (macos): starting with {} paths", paths.len());
        let mut watcher = Watcher::new()?;
        let mut added = HashSet::<&PathBuf>::new();
        let mut synced = true;

        // Store cancellation fd for comparison
        let cancel_fd_orig = cancellation.map(|c| c.as_raw_fd());
        log::debug!(
            "fsmon::imp::run (macos): cancellation fd = {:?}, trigger fd = {:?}",
            cancel_fd_orig,
            trigger_fd
        );

        // Duplicate fds so kqueue can own the duplicates without affecting the originals.
        // The kqueue crate's add_fd takes ownership of the fd, so we must give it duplicates.
        let cancel_fd_dup = match cancel_fd_orig {
            Some(fd) => Some(dup_fd(fd)?),
            None => None,
        };
        let cancel_fd = cancel_fd_dup.as_ref().map(|fd| fd.as_raw_fd());

        let trigger_fd_dup = match trigger_fd {
            Some(fd) => Some(dup_fd(fd)?),
            None => None,
        };
        let trigger_fd_raw = trigger_fd_dup.as_ref().map(|fd| fd.as_raw_fd());

        // Add cancellation fd to kqueue if provided
        if let Some(fd) = cancel_fd {
            watcher.add_fd(fd, EventFilter::EVFILT_READ, FilterFlag::empty())?;
            log::debug!("fsmon::imp::run (macos): added cancellation fd {} to watcher", fd);
        }

        // Add trigger fd to kqueue if provided (for detecting pager exit)
        // We use EVFILT_READ which will signal when the pipe is closed (EOF)
        if let Some(fd) = trigger_fd_raw {
            watcher.add_fd(fd, EventFilter::EVFILT_READ, FilterFlag::empty())?;
            log::debug!(
                "fsmon::imp::run (macos): added trigger fd {} to watcher (dup of {:?})",
                fd,
                trigger_fd
            );
        }

        // Keep the duplicated fds alive - they will be dropped when this function returns
        let _cancel_fd_guard = cancel_fd_dup;
        let _trigger_fd_guard = trigger_fd_dup;

        let flags = FilterFlag::NOTE_FFNOP
            | FilterFlag::NOTE_DELETE
            | FilterFlag::NOTE_WRITE
            | FilterFlag::NOTE_RENAME
            | FilterFlag::NOTE_EXTEND;

        loop {
            log::debug!("fsmon::imp::run (macos): outer loop iteration, synced={}", synced);

            // Re-add cancellation fd on each outer loop iteration since watch() clears the changelist
            if let Some(fd) = cancel_fd {
                watcher.add_fd(fd, EventFilter::EVFILT_READ, FilterFlag::empty())?;
            }

            // Re-add trigger fd on each outer loop iteration
            if let Some(fd) = trigger_fd_raw {
                watcher.add_fd(fd, EventFilter::EVFILT_READ, FilterFlag::empty())?;
            }

            for path in &paths {
                if watcher.add_filename(path, EventFilter::EVFILT_VNODE, flags).is_ok() {
                    added.insert(path);
                    if !synced {
                        log::debug!("fsmon::imp::run (macos): calling handle for path {:?}", path);
                        handle(Event::new(EventKind::Create(CreateKind::Any)).add_path(path.clone()))?;
                    }
                }
            }

            synced = true;
            log::debug!("fsmon::imp::run (macos): calling watcher.watch()");
            watcher.watch()?;
            log::debug!("fsmon::imp::run (macos): watcher.watch() returned");

            while synced {
                log::debug!("fsmon::imp::run (macos): calling poll");
                let event = if let Some(event) = watcher.poll(Some(Duration::from_secs(1))) {
                    event
                } else {
                    log::debug!("fsmon::imp::run (macos): poll timeout, continuing");
                    continue;
                };

                log::debug!("fsmon::imp::run (macos): received kqueue event: {:?}", event);

                // Check if this is a cancellation or trigger event (check by fd)
                match &event.ident {
                    Ident::Fd(fd) if cancel_fd == Some(*fd) => {
                        log::debug!("fsmon::imp::run (macos): cancellation detected via fd match");
                        return Ok(());
                    }
                    Ident::Fd(fd) if trigger_fd_raw == Some(*fd) => {
                        log::debug!("fsmon::imp::run (macos): trigger fd signaled (pager closed), exiting");
                        return Ok(());
                    }
                    _ => {}
                }

                match event {
                    kqueue::Event {
                        data: EventData::Vnode(data),
                        ident: Ident::Filename(_, path),
                    } => {
                        let path = PathBuf::from(path);
                        log::debug!("fsmon::imp::run (macos): vnode event {:?} for {:?}", data, path);
                        let event = match data {
                            Vnode::Delete | Vnode::Revoke => {
                                if added.contains(&path) {
                                    watcher.remove_filename(&path, EventFilter::EVFILT_VNODE)?;
                                    added.remove(&path);
                                }
                                Event::new(EventKind::Remove(RemoveKind::Any)).add_path(path)
                            }
                            Vnode::Write => {
                                if added.len() < paths.len() && path.is_dir() {
                                    synced = false;
                                }
                                Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Any))).add_path(path)
                            }
                            Vnode::Extend | Vnode::Truncate => {
                                Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Size))).add_path(path)
                            }
                            Vnode::Rename => {
                                if added.contains(&path) {
                                    watcher.remove_filename(&path, EventFilter::EVFILT_VNODE)?;
                                    added.remove(&path);
                                }
                                Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Any))).add_path(path)
                            }
                            Vnode::Link => Event::new(EventKind::Create(CreateKind::Any)).add_path(path),
                            Vnode::Attrib => {
                                Event::new(EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any))).add_path(path)
                            }

                            #[allow(unreachable_patterns)]
                            _ => Event::new(EventKind::Other),
                        };
                        log::debug!("fsmon::imp::run (macos): calling handle for event");
                        handle(event)?;
                    }
                    _ => {
                        // Ignore other events (e.g., ReadReady for cancellation that we already handled)
                    }
                };
            }
        }
    }
}
