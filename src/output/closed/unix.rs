use std::{io, mem};

// ---

pub const FD: libc::c_int = libc::STDOUT_FILENO;

/// Whether the standard output is a pipe or a socket.
pub fn watchable() -> bool {
    // SAFETY: `stat` is plain data, so all zeros is a valid value, and `fstat` only writes to it.
    let mut stat: libc::stat = unsafe { mem::zeroed() };
    if unsafe { libc::fstat(FD, &mut stat) } != 0 {
        return false;
    }
    matches!(stat.st_mode & libc::S_IFMT, libc::S_IFIFO | libc::S_IFSOCK)
}

pub fn interrupted() -> bool {
    io::Error::last_os_error().kind() == io::ErrorKind::Interrupted
}
