// std imports
use std::path::PathBuf;

// other local crates
use cancel::CancellationToken;

// local imports
use crate::error::Result;

// ---

pub type Event = notify::Event;
pub type EventKind = notify::EventKind;

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
    cancellation: Option<&CancellationToken>,
) -> Result<()>
where
    H: FnMut(Event) -> Result<()>,
{
    if paths.is_empty() {
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

    imp::run(
        watch,
        |event| {
            if event.paths.iter().any(|path| paths.binary_search(path).is_ok()) {
                handle(event)
            } else {
                Ok(())
            }
        },
        cancellation,
    )
}

#[cfg(all(unix, not(target_os = "macos")))]
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

    const FALLBACK_POLLING_INTERVAL: Duration = Duration::from_secs(1);

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H, cancellation: Option<&CancellationToken>) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher =
            RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

        for path in &paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        if let Some(cancellation) = cancellation {
            run_cancellable(rx, &mut handle, cancellation)
        } else {
            loop {
                match rx.recv() {
                    Ok(Ok(event)) => handle(event)?,
                    Ok(Err(err)) => return Err(err.into()),
                    Err(err) => return Err(Error::RecvTimeoutError { source: err.into() }),
                };
            }
        }
    }

    /// Event-driven cancellable receive loop.
    ///
    /// Uses a bridge thread that forwards mpsc events to a Unix socket,
    /// then `poll()` waits on both the socket and the cancellation pipe fd
    /// for immediate wakeup without busy-waiting.
    fn run_cancellable<H>(
        rx: mpsc::Receiver<notify::Result<Event>>,
        handle: &mut H,
        cancellation: &CancellationToken,
    ) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        let (notify_reader, notify_writer) = UnixStream::pair()?;
        notify_reader.set_nonblocking(true)?;

        let notify_read_fd = notify_reader.as_raw_fd();
        let cancel_fd = cancellation.as_raw_fd();

        let event_queue: Arc<Mutex<VecDeque<notify::Result<Event>>>> = Arc::new(Mutex::new(VecDeque::new()));
        let event_queue_writer = Arc::clone(&event_queue);

        let _bridge = thread::spawn(move || {
            for event in rx {
                if let Ok(mut queue) = event_queue_writer.lock() {
                    queue.push_back(event);
                }
                let _ = (&notify_writer).write(&[1u8]);
            }
        });

        loop {
            let mut pollfds = [
                libc::pollfd {
                    fd: cancel_fd,
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: notify_read_fd,
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];

            let ret = unsafe { libc::poll(pollfds.as_mut_ptr(), 2, -1) };

            if ret < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err.into());
            }

            if pollfds[0].revents & libc::POLLIN != 0 || cancellation.is_cancelled() {
                return Ok(());
            }

            if pollfds[1].revents & libc::POLLIN != 0 {
                // Drain the notification bytes.
                let mut buf = [0u8; 64];
                let _ = std::io::Read::read(&mut &notify_reader, &mut buf);

                // Process all queued events.
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
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::{collections::HashSet, time::Duration};

    use kqueue::{EventData, EventFilter, FilterFlag, Ident, Vnode, Watcher};
    use notify::event::{CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode};

    use super::*;

    fn dup_fd(fd: RawFd) -> io::Result<OwnedFd> {
        let new_fd = unsafe { libc::dup(fd) };
        if new_fd == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(unsafe { OwnedFd::from_raw_fd(new_fd) })
        }
    }

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H, cancellation: Option<&CancellationToken>) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        let mut watcher = Watcher::new()?;
        let mut added = HashSet::<&PathBuf>::new();
        let mut synced = true;

        // Duplicate the cancellation fd so kqueue can own the duplicate
        // without affecting the original.
        let cancel_fd_dup = match cancellation {
            Some(ct) => Some(dup_fd(ct.as_raw_fd())?),
            None => None,
        };
        let cancel_fd = cancel_fd_dup.as_ref().map(|fd| fd.as_raw_fd());

        if let Some(fd) = cancel_fd {
            watcher.add_fd(fd, EventFilter::EVFILT_READ, FilterFlag::empty())?;
        }

        let flags = FilterFlag::NOTE_FFNOP
            | FilterFlag::NOTE_DELETE
            | FilterFlag::NOTE_WRITE
            | FilterFlag::NOTE_RENAME
            | FilterFlag::NOTE_EXTEND;

        loop {
            // Re-add cancellation fd on each outer loop iteration since watch() clears the changelist.
            if let Some(fd) = cancel_fd {
                watcher.add_fd(fd, EventFilter::EVFILT_READ, FilterFlag::empty())?;
            }

            for path in &paths {
                if watcher.add_filename(path, EventFilter::EVFILT_VNODE, flags).is_ok() {
                    added.insert(path);
                    if !synced {
                        handle(Event::new(EventKind::Create(CreateKind::Any)).add_path(path.clone()))?;
                    }
                }
            }

            synced = true;
            watcher.watch()?;

            while synced {
                let event = if let Some(event) = watcher.poll(Some(Duration::from_secs(1))) {
                    event
                } else {
                    continue;
                };

                // Check if this is a cancellation event.
                if let Ident::Fd(fd) = &event.ident {
                    if cancel_fd == Some(*fd) {
                        return Ok(());
                    }
                }

                if let kqueue::Event {
                    data: EventData::Vnode(data),
                    ident: Ident::Filename(_, path),
                } = event
                {
                    let path = PathBuf::from(path);
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
                    handle(event)?;
                }
            }
        }
    }
}
