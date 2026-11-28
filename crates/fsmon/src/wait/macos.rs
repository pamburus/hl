use std::{
    ffi::CString,
    fs::File,
    io,
    os::{
        fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    sync::Arc,
    time::{Duration, Instant},
};

use super::{Entry, Wake};
use crate::stop::Stop;

// ---

/// The changes of the open file that are waited for.
const FILE_EVENTS: u32 = libc::NOTE_WRITE
    | libc::NOTE_EXTEND
    | libc::NOTE_ATTRIB
    | libc::NOTE_DELETE
    | libc::NOTE_RENAME
    | libc::NOTE_REVOKE
    | libc::NOTE_LINK;

/// The changes of a directory that are waited for, which include adding, removing and renaming an entry.
const DIR_EVENTS: u32 = libc::NOTE_WRITE | libc::NOTE_DELETE | libc::NOTE_RENAME | libc::NOTE_REVOKE;

// ---

/// Watches the open file and the directories of the entries with a kqueue of its own.
pub struct Watch {
    queue: OwnedFd,
    /// The descriptors the queue watches, which must stay open while it does.
    _watched: Vec<OwnedFd>,
    stop: Arc<Stop>,
}

impl Watch {
    pub fn new(entries: &[Entry], file: Option<&File>, stop: Arc<Stop>) -> io::Result<Self> {
        // SAFETY: the call takes no arguments.
        let queue = unsafe { libc::kqueue() };
        if queue < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the call succeeded, so the descriptor is open and owned by nothing else.
        let queue = unsafe { OwnedFd::from_raw_fd(queue) };
        crate::stop::set_flag(&queue, libc::F_GETFD, libc::F_SETFD, libc::FD_CLOEXEC)?;

        let mut watched = Vec::with_capacity(entries.len() + 1);
        let mut changes = Vec::with_capacity(entries.len() + 2);
        if let Some(file) = file {
            let fd = file.as_fd().try_clone_to_owned()?;
            changes.push(vnode(&fd, FILE_EVENTS));
            watched.push(fd);
        }
        for (dir, _) in entries {
            let dir = CString::new(dir.as_os_str().as_bytes()).map_err(io::Error::other)?;
            // SAFETY: `dir` is a NUL-terminated string. The directory is opened only to be watched.
            let fd = unsafe { libc::open(dir.as_ptr(), libc::O_EVTONLY | libc::O_CLOEXEC) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: the call succeeded, so the descriptor is open and owned by nothing else.
            let fd = unsafe { OwnedFd::from_raw_fd(fd) };
            changes.push(vnode(&fd, DIR_EVENTS));
            watched.push(fd);
        }
        // Level-triggered, so that every later wait sees the stop too.
        changes.push(event(stop.fd().as_raw_fd(), libc::EVFILT_READ, libc::EV_ADD, 0));

        // SAFETY: the queue is open and `changes` is an array of the length given, with no events to return.
        let n = unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                changes.as_ptr(),
                changes.len() as libc::c_int,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            queue,
            _watched: watched,
            stop,
        })
    }

    pub fn wait(&mut self, timeout: Option<Duration>) -> io::Result<Wake> {
        // A deadline too far to be represented is none.
        let deadline = timeout.and_then(|timeout| Instant::now().checked_add(timeout));
        let stop = self.stop.fd().as_raw_fd() as libc::uintptr_t;
        loop {
            let left = deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
            let timespec = left.map(|left| libc::timespec {
                tv_sec: left.as_secs().min(i32::MAX as u64) as libc::time_t,
                tv_nsec: left.subsec_nanos() as libc::c_long,
            });
            let timespec = timespec
                .as_ref()
                .map_or(std::ptr::null(), |timespec| timespec as *const _);
            // SAFETY: an all-zero `kevent` is a valid value of a plain C struct.
            let mut events: [libc::kevent; 8] = unsafe { std::mem::zeroed() };
            // SAFETY: the queue is open, `events` is an array of the length given, and `timespec` is null or valid.
            let n = unsafe {
                libc::kevent(
                    self.queue.as_raw_fd(),
                    std::ptr::null(),
                    0,
                    events.as_mut_ptr(),
                    events.len() as libc::c_int,
                    timespec,
                )
            };
            if n < 0 {
                let err = io::Error::last_os_error();
                if err.kind() != io::ErrorKind::Interrupted {
                    return Err(err);
                }
            } else if n > 0 {
                let events = &events[..n as usize];
                if self.stop.is_stopped() || events.iter().any(|event| event.ident == stop) {
                    return Ok(Wake::Stopped);
                }
                return Ok(Wake::Ready);
            }
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return Ok(Wake::Timeout);
            }
        }
    }
}

fn vnode(fd: &OwnedFd, fflags: u32) -> libc::kevent {
    event(
        fd.as_raw_fd(),
        libc::EVFILT_VNODE,
        libc::EV_ADD | libc::EV_CLEAR,
        fflags,
    )
}

fn event(fd: libc::c_int, filter: i16, flags: u16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident: fd as libc::uintptr_t,
        filter,
        flags,
        fflags,
        data: 0,
        udata: std::ptr::null_mut(),
    }
}
