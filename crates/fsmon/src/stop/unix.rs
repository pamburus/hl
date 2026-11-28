use std::{
    io,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    time::Duration,
};

use super::Stop;
use crate::wait::{Wake, unix::wait_readable};

// ---

/// A pipe whose read end becomes readable once stopped, and stays readable.
pub struct Waker {
    read: OwnedFd,
    write: OwnedFd,
}

impl Waker {
    pub fn new() -> io::Result<Self> {
        let mut fds = [0; 2];
        // SAFETY: `fds` is an array of two descriptors for the call to fill.
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the call succeeded, so both descriptors are open and owned by nothing else.
        let (read, write) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        for fd in [&read, &write] {
            set_flag(fd, libc::F_GETFD, libc::F_SETFD, libc::FD_CLOEXEC)?;
            set_flag(fd, libc::F_GETFL, libc::F_SETFL, libc::O_NONBLOCK)?;
        }
        Ok(Self { read, write })
    }

    /// Makes the read end readable for good, as nothing reads from it.
    pub fn wake(&self) {
        // A failure leaves nothing to do, and the pipe cannot be full, as only one byte is ever written.
        // SAFETY: the descriptor is open and the buffer is one byte long.
        unsafe { libc::write(self.write.as_raw_fd(), [1u8].as_ptr().cast(), 1) };
    }

    pub fn fd(&self) -> BorrowedFd<'_> {
        self.read.as_fd()
    }
}

pub fn sleep(stop: &Stop, timeout: Duration) -> io::Result<Wake> {
    let ready = wait_readable(&[stop.fd()], Some(timeout))?;
    Ok(if ready != 0 || stop.is_stopped() {
        Wake::Stopped
    } else {
        Wake::Timeout
    })
}

pub fn set_flag(fd: &impl AsRawFd, get: libc::c_int, set: libc::c_int, flag: libc::c_int) -> io::Result<()> {
    let fd = fd.as_raw_fd();
    // SAFETY: the descriptor is open, and the commands take and return plain integers.
    unsafe {
        let flags = libc::fcntl(fd, get);
        if flags < 0 || libc::fcntl(fd, set, flags | flag) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}
