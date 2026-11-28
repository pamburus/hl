use std::{ffi::CString, os::unix::ffi::OsStrExt, path::Path};

use super::Reliability;

// ---

/// Magic numbers of file systems whose changes are all made on this machine, so inotify sees each of them.
///
/// Any other file system, including network and FUSE file systems, is not confirmed, and so are proc and sysfs, whose
/// files change with no event that inotify sees.
const KNOWN_LOCAL: &[u32] = &[
    0x0000_EF53, // ext2, ext3, ext4
    0x9123_683E, // btrfs
    0x5846_5342, // xfs
    0xF2F5_2010, // f2fs
    0xCA45_1A4E, // bcachefs
    0x5265_4973, // reiserfs
    0x2FC1_2FC1, // zfs
    0x3434,      // nilfs
    0x0000_4D44, // msdos, vfat
    0x2011_BAB0, // exfat
    0x5346_544E, // ntfs
    0x7366_746E, // ntfs3
    0x0000_9660, // iso9660
    0x1501_3346, // udf
    0x0000_72B6, // jffs2
    0x2405_1905, // ubifs
    0x0000_137F, // minix
    0x0000_138F, // minix, 30-character names
    0x0000_2468, // minix v2
    0x0000_2478, // minix v2, 30-character names
    0x4D5A,      // minix v3
    0x0102_1994, // tmpfs
    0x8584_58F6, // ramfs
    0x9584_58F6, // hugetlbfs
    0x794C_7630, // overlayfs
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
    // The field is signed and of a different width on some targets, and the magic numbers are 32-bit.
    #[allow(clippy::unnecessary_cast)]
    let magic = buf.f_type as u32;
    if KNOWN_LOCAL.contains(&magic) {
        Reliability::KnownLocal
    } else {
        Reliability::NotConfirmed
    }
}
