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

pub fn run_with_cancellation<H>(mut paths: Vec<PathBuf>, mut handle: H, cancellation: Option<&Cancellation>) -> Result<()>
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

    imp::run(watch, |event| {
        log::debug!("fsmon::run: received event: {:?}", event.kind);
        if event.paths.iter().any(|path| paths.binary_search(path).is_ok()) {
            handle(event)
        } else {
            Ok(())
        }
    }, cancellation.map(|c| &c.inner), trigger_fd)
}

// Platform-specific cancellation implementations
#[cfg(unix)]
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

// Non-macOS implementation using notify's RecommendedWatcher
#[cfg(not(target_os = "macos"))]
mod imp {
    use std::sync::mpsc::{self};
    use std::time::Duration;

    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    use super::*;
    use crate::error::Error;

    pub use super::cancellation::Cancellation;

    const FALLBACK_POLLING_INTERVAL: Duration = Duration::from_secs(1);

    #[cfg(unix)]
    type TriggerFd = std::os::unix::io::RawFd;
    #[cfg(not(unix))]
    type TriggerFd = ();

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H, cancellation: Option<&Cancellation>, _trigger_fd: Option<TriggerFd>) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        // TODO: Use poll() to properly monitor trigger_fd on non-macOS Unix platforms
        // For now, we rely on the cancellation mechanism

        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

        for path in &paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        // If we have a cancellation token, we need to check it periodically
        // since std::sync::mpsc doesn't expose a pollable fd.
        if let Some(cancellation) = cancellation {
            loop {
                // First, check if cancellation was requested
                if cancellation.is_cancelled() {
                    log::debug!("fsmon::imp::run: cancellation detected");
                    return Ok(());
                }

                // Try to receive with a short timeout, then check cancellation again
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(event)) => handle(event)?,
                    Ok(Err(err)) => return Err(err.into()),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Loop will check cancellation at the top
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        return Err(Error::RecvTimeoutError {
                            source: std::sync::mpsc::RecvTimeoutError::Disconnected.into(),
                        });
                    }
                }
            }
        } else {
            // Original behavior without cancellation
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

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H, cancellation: Option<&Cancellation>, trigger_fd: Option<RawFd>) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        log::debug!("fsmon::imp::run (macos): starting with {} paths", paths.len());
        let mut watcher = Watcher::new()?;
        let mut added = HashSet::<&PathBuf>::new();
        let mut synced = true;

        // Store cancellation fd for comparison
        let cancel_fd_orig = cancellation.map(|c| c.as_raw_fd());
        log::debug!("fsmon::imp::run (macos): cancellation fd = {:?}, trigger fd = {:?}", cancel_fd_orig, trigger_fd);

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
            log::debug!("fsmon::imp::run (macos): added trigger fd {} to watcher (dup of {:?})", fd, trigger_fd);
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
