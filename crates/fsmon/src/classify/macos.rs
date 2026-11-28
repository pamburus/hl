use std::{
    ffi::{CStr, CString},
    os::unix::ffi::OsStrExt,
    path::Path,
};

use super::Reliability;

// ---

/// Names of file systems whose changes are all made on this machine, so kqueue sees each of them.
///
/// Any other file system, including network and FUSE file systems, is not confirmed.
const KNOWN_LOCAL: &[&[u8]] = &[
    b"apfs", b"hfs", b"msdos", b"exfat", b"cd9660", b"udf", b"ntfs", b"tmpfs", b"devfs",
];

pub fn classify(path: &Path) -> Reliability {
    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return Reliability::NotConfirmed;
    };
    // SAFETY: an all-zero `statfs` is a valid value of a plain C struct.
    let mut buf: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: `path` is a NUL-terminated string and `buf` is a valid `statfs` to fill.
    if unsafe { libc::statfs(path.as_ptr(), &mut buf) } != 0 {
        return Reliability::NotConfirmed;
    }
    // SAFETY: the kernel fills the name as a NUL-terminated string within the array.
    let name = unsafe { CStr::from_ptr(buf.f_fstypename.as_ptr()) }.to_bytes();
    if KNOWN_LOCAL.iter().any(|local| local.eq_ignore_ascii_case(name)) {
        Reliability::KnownLocal
    } else {
        Reliability::NotConfirmed
    }
}
