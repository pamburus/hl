use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
    time::Duration,
};

// ---

/// Waits until any of the descriptors is readable, or the timeout passes, or a signal interrupts the wait.
///
/// Returns a mask with bit `i` set if `fds[i]` is readable, which is empty after a timeout or an interruption.
/// A descriptor whose other end is closed counts as readable, as a read from it does not block.
pub fn wait_readable(fds: &[BorrowedFd<'_>], timeout: Option<Duration>) -> io::Result<u32> {
    debug_assert!(fds.len() <= 32);
    #[cfg(target_os = "macos")]
    if fds.iter().all(|fd| (fd.as_raw_fd() as usize) < libc::FD_SETSIZE) {
        // The poll call of macOS does not support terminals, which standard input often is.
        return select(fds, timeout);
    }
    poll(fds, timeout)
}

fn poll(fds: &[BorrowedFd<'_>], timeout: Option<Duration>) -> io::Result<u32> {
    let mut polled: Vec<_> = fds
        .iter()
        .map(|fd| libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        })
        .collect();
    let timeout = match timeout {
        // Rounded up, so that a short timeout does not become a busy loop.
        Some(timeout) => timeout.as_nanos().div_ceil(1_000_000).min(i32::MAX as u128) as libc::c_int,
        None => -1,
    };
    // SAFETY: `polled` is an array of the length given.
    let n = unsafe { libc::poll(polled.as_mut_ptr(), polled.len() as libc::nfds_t, timeout) };
    if n < 0 {
        return interrupted();
    }
    Ok(polled.iter().enumerate().fold(0, |mask, (i, fd)| {
        let ready = fd.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0;
        mask | (u32::from(ready) << i)
    }))
}

#[cfg(target_os = "macos")]
fn select(fds: &[BorrowedFd<'_>], timeout: Option<Duration>) -> io::Result<u32> {
    // SAFETY: an all-zero `fd_set` is an empty set.
    let mut set: libc::fd_set = unsafe { std::mem::zeroed() };
    let mut max = 0;
    for fd in fds {
        // SAFETY: the descriptor is below `FD_SETSIZE`, which the caller checks.
        unsafe { libc::FD_SET(fd.as_raw_fd(), &mut set) };
        max = max.max(fd.as_raw_fd());
    }
    let mut timeval = timeout.map(|timeout| {
        // Rounded up, so that a short timeout does not become a busy loop.
        let micros = timeout.as_nanos().div_ceil(1000).min(i32::MAX as u128 * 1_000_000);
        libc::timeval {
            tv_sec: (micros / 1_000_000) as libc::time_t,
            tv_usec: (micros % 1_000_000) as libc::suseconds_t,
        }
    });
    let timeval = timeval
        .as_mut()
        .map_or(std::ptr::null_mut(), |timeval| timeval as *mut _);
    // SAFETY: `set` holds descriptors up to `max`, and `timeval` is null or a valid timeout.
    let n = unsafe { libc::select(max + 1, &mut set, std::ptr::null_mut(), std::ptr::null_mut(), timeval) };
    if n < 0 {
        return interrupted();
    }
    Ok(fds.iter().enumerate().fold(0, |mask, (i, fd)| {
        // SAFETY: the descriptor is below `FD_SETSIZE`.
        let ready = unsafe { libc::FD_ISSET(fd.as_raw_fd(), &set) };
        mask | (u32::from(ready) << i)
    }))
}

/// Treats an interruption as a wake with nothing ready, and any other failure as an error.
fn interrupted() -> io::Result<u32> {
    let err = io::Error::last_os_error();
    if err.kind() == io::ErrorKind::Interrupted {
        Ok(0)
    } else {
        Err(err)
    }
}
