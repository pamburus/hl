// std imports
use std::path::PathBuf;

// local imports
use crate::error::Result;

// ---

pub type Event = notify::Event;
pub type EventKind = notify::EventKind;

// ---

pub struct Monitor {
    cancel_token: Option<imp::CancelToken>,
}

impl Monitor {
    pub fn new() -> Self {
        Self { cancel_token: None }
    }

    pub fn cancellable(self) -> std::io::Result<(Self, CancelHandle)> {
        let (token, handle) = imp::create_cancel_pair()?;
        Ok((Self { cancel_token: Some(token) }, CancelHandle { inner: handle }))
    }

    pub fn try_clone(&self) -> std::io::Result<Self> {
        Ok(Self {
            cancel_token: self.cancel_token.as_ref().map(|t| t.try_clone()).transpose()?,
        })
    }

    pub fn run<H>(self, paths: Vec<PathBuf>, handle: H) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        let (paths, watch) = prepare_paths(paths);

        if paths.is_empty() {
            return Ok(());
        }

        imp::run(watch, into_filter(paths, handle), self.cancel_token)
    }
}

// ---

pub struct CancelHandle {
    inner: imp::CancelHandle,
}

impl CancelHandle {
    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

// ---

fn prepare_paths(mut paths: Vec<PathBuf>) -> (Vec<PathBuf>, Vec<PathBuf>) {
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

    (paths, watch)
}

fn into_filter<H>(paths: Vec<PathBuf>, mut handle: H) -> impl FnMut(Event) -> Result<()>
where
    H: FnMut(Event) -> Result<()>,
{
    move |event| {
        if event.paths.iter().any(|path| paths.binary_search(path).is_ok()) {
            handle(event)
        } else {
            Ok(())
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::Duration;

    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    use super::*;
    use crate::error::Error;

    const FALLBACK_POLLING_INTERVAL: Duration = Duration::from_secs(1);

    pub use self::platform::*;

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H, cancel: Option<CancelToken>) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher =
            RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_INTERVAL))?;

        for path in &paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        #[cfg(unix)]
        if let Some(cancel) = cancel {
            return run_cancellable(rx, &mut handle, cancel);
        }

        #[cfg(not(unix))]
        let _ = cancel;

        run_simple(rx, &mut handle)
    }

    fn run_simple<H>(rx: mpsc::Receiver<notify::Result<Event>>, handle: &mut H) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        loop {
            match rx.recv() {
                Ok(Ok(event)) => handle(event)?,
                Ok(Err(err)) => return Err(err.into()),
                Err(err) => return Err(Error::RecvTimeoutError { source: err.into() }),
            };
        }
    }

    #[cfg(unix)]
    fn run_cancellable<H>(
        rx: mpsc::Receiver<notify::Result<Event>>,
        handle: &mut H,
        cancel: CancelToken,
    ) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        use std::collections::VecDeque;
        use std::io::Write;
        use std::os::unix::io::AsRawFd;
        use std::os::unix::net::UnixStream;
        use std::sync::{Arc, Mutex};
        use std::thread;

        let (notify_reader, notify_writer) = UnixStream::pair()?;
        notify_reader.set_nonblocking(true)?;

        let notify_read_fd = notify_reader.as_raw_fd();
        let cancel_raw_fd = cancel.0.as_raw_fd();

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
                    fd: cancel_raw_fd,
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

            if pollfds[0].revents & libc::POLLIN != 0 {
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

    #[cfg(unix)]
    mod platform {
        use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
        use std::sync::atomic::{AtomicBool, Ordering};

        pub struct CancelToken(pub(super) OwnedFd);

        pub struct CancelHandle {
            cancelled: AtomicBool,
            write_fd: OwnedFd,
        }

        // Safety: write_fd is only written to once, guarded by the AtomicBool.
        unsafe impl Sync for CancelHandle {}

        impl CancelToken {
            pub fn try_clone(&self) -> std::io::Result<Self> {
                let new_fd = unsafe { libc::dup(self.0.as_raw_fd()) };
                if new_fd == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(Self(unsafe { OwnedFd::from_raw_fd(new_fd) }))
            }
        }

        impl CancelHandle {
            pub fn cancel(&self) {
                if !self.cancelled.swap(true, Ordering::SeqCst) {
                    unsafe {
                        libc::write(self.write_fd.as_raw_fd(), [1u8].as_ptr() as *const libc::c_void, 1);
                    }
                }
            }
        }

        pub fn create_cancel_pair() -> std::io::Result<(CancelToken, CancelHandle)> {
            let mut fds = [0i32; 2];
            if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
                return Err(std::io::Error::last_os_error());
            }
            let (read_fd, write_fd) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
            Ok((
                CancelToken(read_fd),
                CancelHandle {
                    cancelled: AtomicBool::new(false),
                    write_fd,
                },
            ))
        }
    }

    #[cfg(not(unix))]
    mod platform {
        pub struct CancelToken;

        pub struct CancelHandle;

        impl CancelToken {
            pub fn try_clone(&self) -> std::io::Result<Self> {
                Ok(Self)
            }
        }

        impl CancelHandle {
            pub fn cancel(&self) {}
        }

        pub fn create_cancel_pair() -> std::io::Result<(CancelToken, CancelHandle)> {
            Ok((CancelToken, CancelHandle))
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::{collections::HashSet, time::Duration};

    use kqueue::{EventData, EventFilter, FilterFlag, Ident, Vnode, Watcher};
    use notify::event::{CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode};

    use super::*;

    pub struct CancelToken(OwnedFd);

    pub struct CancelHandle {
        cancelled: AtomicBool,
        write_fd: OwnedFd,
    }

    // Safety: write_fd is only written to once, guarded by the AtomicBool.
    unsafe impl Sync for CancelHandle {}

    impl CancelToken {
        pub fn try_clone(&self) -> io::Result<Self> {
            Ok(Self(dup_fd(self.0.as_raw_fd())?))
        }
    }

    impl CancelHandle {
        pub fn cancel(&self) {
            if !self.cancelled.swap(true, Ordering::SeqCst) {
                unsafe {
                    libc::write(self.write_fd.as_raw_fd(), [1u8].as_ptr() as *const libc::c_void, 1);
                }
            }
        }
    }

    pub fn create_cancel_pair() -> io::Result<(CancelToken, CancelHandle)> {
        let mut fds = [0i32; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
            return Err(io::Error::last_os_error());
        }
        let (read_fd, write_fd) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        Ok((
            CancelToken(read_fd),
            CancelHandle {
                cancelled: AtomicBool::new(false),
                write_fd,
            },
        ))
    }

    fn dup_fd(fd: RawFd) -> io::Result<OwnedFd> {
        let new_fd = unsafe { libc::dup(fd) };
        if new_fd == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(unsafe { OwnedFd::from_raw_fd(new_fd) })
        }
    }

    pub fn run<H>(paths: Vec<PathBuf>, mut handle: H, cancel_token: Option<CancelToken>) -> Result<()>
    where
        H: FnMut(Event) -> Result<()>,
    {
        let mut watcher = Watcher::new()?;
        let mut added = HashSet::<&PathBuf>::new();
        let mut synced = true;

        // Duplicate the cancel fd so kqueue can own the duplicate
        // without affecting the original.
        let cancel_fd_dup = cancel_token.as_ref().map(|fd| dup_fd(fd.0.as_raw_fd())).transpose()?;
        let cancel_fd_raw = cancel_fd_dup.as_ref().map(|fd| fd.as_raw_fd());

        if let Some(fd) = cancel_fd_raw {
            watcher.add_fd(fd, EventFilter::EVFILT_READ, FilterFlag::empty())?;
        }

        let flags = FilterFlag::NOTE_FFNOP
            | FilterFlag::NOTE_DELETE
            | FilterFlag::NOTE_WRITE
            | FilterFlag::NOTE_RENAME
            | FilterFlag::NOTE_EXTEND;

        loop {
            // Re-add cancel fd on each outer loop iteration since watch() clears the changelist.
            if let Some(fd) = cancel_fd_raw {
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
                    if cancel_fd_raw == Some(*fd) {
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
