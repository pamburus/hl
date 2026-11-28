pub use super::unix::watchable;
use super::unix::{FD, interrupted};

// ---

/// Returns true once the standard output is closed by its reader, or false if that cannot be waited for.
///
/// Errors and hang-ups are reported by `poll` even with no events requested, so it waits only for them, without
/// waking up when the output becomes writable.
pub fn wait_closed() -> bool {
    loop {
        let mut fd = libc::pollfd {
            fd: FD,
            events: 0,
            revents: 0,
        };
        // SAFETY: `fd` is a valid `pollfd` for the duration of the call.
        if unsafe { libc::poll(&mut fd, 1, -1) } < 0 {
            if interrupted() {
                continue;
            }
            return false;
        }
        if fd.revents & libc::POLLNVAL != 0 {
            return false;
        }
        if fd.revents & (libc::POLLERR | libc::POLLHUP) != 0 {
            return true;
        }
    }
}
