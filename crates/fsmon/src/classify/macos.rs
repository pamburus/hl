use std::path::Path;

use super::Reliability;

// ---

/// Known-local filesystem type names on macOS.
const KNOWN_LOCAL: &[&str] = &[
    "apfs", "hfs", "msdos", "exfat", "cd9660", "udf", "ntfs", "tmpfs", "devfs", "autofs", "ramfs",
];

pub fn classify(path: &Path) -> Reliability {
    use libc::statfs;
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = match CString::new(path.as_os_str().as_bytes()) {
        Ok(p) => p,
        Err(_) => return Reliability::NotConfirmed,
    };

    let mut buf: libc::statfs = unsafe { std::mem::zeroed() };
    let ret = unsafe { statfs(c_path.as_ptr(), &mut buf) };
    if ret != 0 {
        return Reliability::NotConfirmed;
    }

    let f_type = unsafe {
        let bytes = &buf.f_fstypename as *const libc::c_char;
        std::ffi::CStr::from_ptr(bytes)
            .to_str()
            .unwrap_or("")
            .to_ascii_lowercase()
    };

    if KNOWN_LOCAL.contains(&f_type.as_str()) {
        Reliability::KnownLocal
    } else {
        Reliability::NotConfirmed
    }
}
