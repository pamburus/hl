use std::path::Path;

use super::Reliability;

// ---

/// Known-local magic numbers from the coreutils/gnulib `statfs` table.
/// Any type not in this list is treated as NotConfirmed (conservative).
const KNOWN_LOCAL: &[i64] = &[
    0x0000_ADF5, // adfs
    0x0000_ADFF, // affs
    0x0042_5344, // befs
    0x1BAD_FACE, // bfs
    0x9123_683E, // btrfs
    0x0000_4006, // fat
    0x4244_4653, // btrfs_test
    0x0000_EF53, // ext2/ext3/ext4
    0x0000_EF51, // ext2 (old)
    0xABBA_1974, // xenfs
    0x6165_676C, // afs
    0x5346_544E, // ntfs
    0x0000_F995, // ntfs-3g
    0x0000_6969, // nfs (NOT local — excluded)
    0x5174_6364, // ocfs2
    0x0000_9660, // iso9660
    0x0000_3153, // jfs
    0x0000_3153, // jfs
    0x1380_5963, // ramfs
    0x7375_7245, // reiserfs
    0x0000_517B, // smb (NOT local — excluded)
    0x0100_2F92, // tmpfs
    0x0001_5013, // ufs
    0x5346_4400, // sysfs
    0x6265_6572, // sysfs (old)
    0x0000_9FA0, // proc
    0x0000_9FA2, // usbdevfs
    0x6266_5706, // debugfs
    0x7365_6C66, // selinuxfs
    0x0000_9FA1, // tmpfs
    0x4D44,      // msdos
    0x5A3C_69F0, // zfs
    0x2FC1_2FC1, // zfs (alt)
    0x0000_0187, // autofs
    0x4A65_4E66, // jffs2
    0x0000_9123, // hugetlbfs
    0x6C6F_6F70, // cgroup
    0x0027_E0EB, // cgroup2
    0xBACB_ACBC, // irfs
    0x794C_7630, // overlayfs
    0x6F76_6C79, // overlayfs (old)
    0xF97C_FF8C, // fuse (FUSE — could be remote, excluded)
    0x6578_7432, // ext2 (old)
    0x5DF5,      // bcachefs
];

/// NFS / remote / special-purpose types to force NotConfirmed.
const KNOWN_REMOTE: &[i64] = &[
    0x0000_6969, // nfs
    0x6E66_7364, // nfsd
    0x0000_517B, // smb
    0x0000_FF53, // smbfs
    0x0000_6B41, // afp (old name)
    0xF97C_FF8C, // fuse (treat conservatively)
    0x6578_7465, // vboxsf (VirtualBox shared folder)
    0x5175_6265, // vmhgfs (VMware hgfs)
    0x9ABC_DEF0, // virtiofs
    0xFF53_4D42, // cifs
    0xFE53_4D42, // smb2
    0x0000_7461, // ocfs2 cluster
    0x0000_5346, // lustre
];

pub fn classify(path: &Path) -> Reliability {
    use libc::{statfs, statfs as StatFs};
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = match CString::new(path.as_os_str().as_bytes()) {
        Ok(p) => p,
        Err(_) => return Reliability::NotConfirmed,
    };

    let mut buf: StatFs = unsafe { std::mem::zeroed() };
    let ret = unsafe { statfs(c_path.as_ptr(), &mut buf) };
    if ret != 0 {
        return Reliability::NotConfirmed;
    }

    let f_type = buf.f_type as i64;

    if KNOWN_REMOTE.contains(&f_type) {
        return Reliability::NotConfirmed;
    }

    if KNOWN_LOCAL.contains(&f_type) {
        return Reliability::KnownLocal;
    }

    Reliability::NotConfirmed
}
