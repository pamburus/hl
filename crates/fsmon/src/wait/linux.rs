use std::{
    collections::HashMap,
    ffi::CString,
    fs::File,
    io,
    os::{
        fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    sync::{Arc, Mutex, MutexGuard, PoisonError, Weak},
    time::{Duration, Instant},
};

use super::{Entry, Wake, unix::wait_readable};
use crate::stop::Stop;

// ---

/// The events of a directory that may change a file in it, or the file a name in it refers to.
const MASK: u32 = libc::IN_MODIFY
    | libc::IN_ATTRIB
    | libc::IN_CREATE
    | libc::IN_MOVED_FROM
    | libc::IN_MOVED_TO
    | libc::IN_DELETE
    | libc::IN_DELETE_SELF
    | libc::IN_MOVE_SELF;

/// The size of the fixed part of an event, which is followed by the name.
const HEADER: usize = std::mem::size_of::<libc::inotify_event>();

/// The inotify instance the watches share while any of them exists.
static SHARED: Mutex<Weak<Shared>> = Mutex::new(Weak::new());

// ---

/// Watches the directories of the entries.
///
/// All watches share one inotify instance, as a user may have only 128 of them by default. Each watch has an event
/// descriptor of its own, which the wait of any watch that reads an event about the watch's entries signals.
pub struct Watch {
    shared: Arc<Shared>,
    id: u64,
    event: Arc<OwnedFd>,
    stop: Arc<Stop>,
}

impl Watch {
    pub fn new(entries: &[Entry], _file: Option<&File>, stop: Arc<Stop>) -> io::Result<Self> {
        let shared = Shared::get()?;
        let event = Arc::new(eventfd()?);
        let id = shared.subscribe(entries, &event)?;
        Ok(Self {
            shared,
            id,
            event,
            stop,
        })
    }

    pub fn wait(&mut self, timeout: Option<Duration>) -> io::Result<Wake> {
        // A deadline too far to be represented is none.
        let deadline = timeout.and_then(|timeout| Instant::now().checked_add(timeout));
        loop {
            let left = deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
            let ready = wait_readable(&[self.shared.fd.as_fd(), self.event.as_fd(), self.stop.fd()], left)?;
            if ready & 0b100 != 0 || self.stop.is_stopped() {
                return Ok(Wake::Stopped);
            }
            if ready & 0b001 != 0 {
                self.shared.drain()?;
            }
            // The event descriptor is checked after the events are read, as reading them may have signalled it.
            if ready != 0 && take(&self.event)? {
                return Ok(Wake::Ready);
            }
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return Ok(Wake::Timeout);
            }
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.shared.unsubscribe(self.id);
    }
}

// ---

/// The inotify instance and the watches that use it.
struct Shared {
    fd: OwnedFd,
    state: Mutex<State>,
}

struct State {
    next_id: u64,
    /// The entries of the watches in each watched directory, by its watch descriptor.
    subs: HashMap<libc::c_int, Vec<Sub>>,
    buf: Vec<u8>,
}

/// An entry of a watch, as its name in the directory and the event descriptor of the watch.
struct Sub {
    id: u64,
    name: Vec<u8>,
    event: Arc<OwnedFd>,
}

impl Shared {
    /// Returns the shared instance, which is created if no watch exists.
    fn get() -> io::Result<Arc<Self>> {
        let mut shared = SHARED.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(shared) = shared.upgrade() {
            return Ok(shared);
        }
        // SAFETY: the call takes only flags.
        let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let new = Arc::new(Self {
            // SAFETY: the call succeeded, so the descriptor is open and owned by nothing else.
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
            state: Mutex::new(State {
                next_id: 0,
                subs: HashMap::new(),
                buf: vec![0; 64 * 1024],
            }),
        });
        *shared = Arc::downgrade(&new);
        Ok(new)
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Watches the directories of the entries of a new watch, and returns the id of the watch.
    fn subscribe(&self, entries: &[Entry], event: &Arc<OwnedFd>) -> io::Result<u64> {
        let mut state = self.state();
        let id = state.next_id;
        state.next_id += 1;
        for (dir, name) in entries {
            match self.add(dir.as_os_str().as_bytes()) {
                Ok(wd) => state.subs.entry(wd).or_default().push(Sub {
                    id,
                    name: name.as_bytes().to_owned(),
                    event: event.clone(),
                }),
                Err(err) => {
                    self.remove(&mut state, id);
                    return Err(err);
                }
            }
        }
        Ok(id)
    }

    /// Watches a directory, and returns its watch descriptor, which is the one it has if it is watched already.
    fn add(&self, dir: &[u8]) -> io::Result<libc::c_int> {
        let dir = CString::new(dir).map_err(io::Error::other)?;
        // The mask is the same for every directory, so watching one again does not change it.
        // SAFETY: the descriptor is open and `dir` is a NUL-terminated string.
        let wd = unsafe { libc::inotify_add_watch(self.fd.as_raw_fd(), dir.as_ptr(), MASK) };
        if wd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(wd)
    }

    fn unsubscribe(&self, id: u64) {
        self.remove(&mut self.state(), id);
    }

    /// Removes the entries of a watch, and stops watching the directories no other watch has entries in.
    fn remove(&self, state: &mut State, id: u64) {
        state.subs.retain(|&wd, subs| {
            subs.retain(|sub| sub.id != id);
            if subs.is_empty() {
                // A failure means the directory is not watched any more, which is the goal.
                // SAFETY: the descriptor is open and the call takes plain integers.
                unsafe { libc::inotify_rm_watch(self.fd.as_raw_fd(), wd) };
            }
            !subs.is_empty()
        });
    }

    /// Reads a buffer of the pending events and signals the watches they are about.
    ///
    /// The events that do not fit in the buffer keep the descriptor readable, so they are read on the next wake, after
    /// the waiter checks for a stop and for its own events, which a steady stream of events would otherwise put off.
    fn drain(&self) -> io::Result<()> {
        let mut state = self.state();
        let State { subs, buf, .. } = &mut *state;
        let n = loop {
            // SAFETY: the descriptor is open and the buffer is of the length given.
            let n = unsafe { libc::read(self.fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len()) };
            if n >= 0 {
                break n as usize;
            }
            let err = io::Error::last_os_error();
            match err.kind() {
                io::ErrorKind::WouldBlock => return Ok(()),
                io::ErrorKind::Interrupted => continue,
                _ => return Err(err),
            }
        };
        let mut events = &buf[..n];
        while events.len() >= HEADER {
            // SAFETY: the kernel writes whole events, and the header may be unaligned within the buffer.
            let event: libc::inotify_event = unsafe { std::ptr::read_unaligned(events.as_ptr().cast()) };
            let end = (HEADER + event.len as usize).min(events.len());
            let name = &events[HEADER..end];
            let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
            if event.mask & libc::IN_Q_OVERFLOW != 0 {
                // Events were lost, so any watch may have missed one.
                subs.values().flatten().for_each(|sub| signal(&sub.event));
            } else if let Some(subs) = subs.get(&event.wd) {
                // An event without a name is about the directory itself.
                subs.iter()
                    .filter(|sub| name.is_empty() || sub.name == name)
                    .for_each(|sub| signal(&sub.event));
            }
            events = &events[end..];
        }
        Ok(())
    }
}

// ---

fn eventfd() -> io::Result<OwnedFd> {
    // SAFETY: the call takes an initial value and flags.
    let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the call succeeded, so the descriptor is open and owned by nothing else.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Makes an event descriptor readable until it is taken.
fn signal(event: &OwnedFd) {
    // A failure leaves nothing to do, as the counter cannot overflow by adding one at a time.
    // SAFETY: the descriptor is open and the buffer is eight bytes long.
    unsafe { libc::write(event.as_raw_fd(), 1u64.to_ne_bytes().as_ptr().cast(), 8) };
}

/// Tells whether an event descriptor was signalled, and resets it.
fn take(event: &OwnedFd) -> io::Result<bool> {
    let mut value = 0u64;
    loop {
        // SAFETY: the descriptor is open and the buffer is eight bytes long.
        let n = unsafe { libc::read(event.as_raw_fd(), (&raw mut value).cast(), 8) };
        if n >= 0 {
            return Ok(true);
        }
        let err = io::Error::last_os_error();
        match err.kind() {
            io::ErrorKind::WouldBlock => return Ok(false),
            io::ErrorKind::Interrupted => continue,
            _ => return Err(err),
        }
    }
}
