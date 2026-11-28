use std::{
    mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    ptr,
};

pub use super::unix::watchable;
use super::unix::{FD, interrupted};

// ---

/// Returns true once the standard output is closed by its reader, or false if that cannot be waited for.
///
/// Here `poll` reports no hang-up unless the output is asked to become writable, which it nearly always is, so the
/// write filter of a kernel queue is used instead: it reports the end of the output, and otherwise wakes up only
/// when the reader takes some of what was written.
pub fn wait_closed() -> bool {
    // SAFETY: `kqueue` takes no arguments, and the descriptor it returns is owned here.
    let queue = unsafe { libc::kqueue() };
    if queue < 0 {
        return false;
    }
    // SAFETY: `queue` is a new descriptor owned by nothing else.
    let queue = unsafe { OwnedFd::from_raw_fd(queue) };
    // SAFETY: `kevent` is plain data, so all zeros is a valid value.
    let mut event: libc::kevent = unsafe { mem::zeroed() };
    event.ident = FD as _;
    event.filter = libc::EVFILT_WRITE;
    event.flags = libc::EV_ADD | libc::EV_CLEAR;
    // SAFETY: the change list is one valid event, and no events are received.
    if unsafe { libc::kevent(queue.as_raw_fd(), &event, 1, ptr::null_mut(), 0, ptr::null()) } < 0 {
        return false;
    }
    loop {
        // SAFETY: as above.
        let mut received: libc::kevent = unsafe { mem::zeroed() };
        // SAFETY: there are no changes, and at most one event is written to `received`.
        let n = unsafe { libc::kevent(queue.as_raw_fd(), ptr::null(), 0, &mut received, 1, ptr::null()) };
        if n < 0 {
            if interrupted() {
                continue;
            }
            return false;
        }
        // Without a timeout, an event is always received.
        if received.flags & libc::EV_ERROR != 0 {
            return false;
        }
        if received.flags & libc::EV_EOF != 0 {
            return true;
        }
    }
}
