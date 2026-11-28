use std::{
    ffi::OsString,
    os::windows::ffi::OsStrExt,
    path::{Component, Path, Prefix},
};

use windows_sys::Win32::{
    Storage::FileSystem::GetDriveTypeW,
    System::WindowsProgramming::{DRIVE_NO_ROOT_DIR, DRIVE_REMOTE, DRIVE_UNKNOWN},
};

use super::Reliability;

// ---

pub fn classify(path: &Path) -> Reliability {
    let root = match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                let mut root = OsString::from(char::from(letter).to_string());
                root.push(":\\");
                Some(root)
            }
            // A share on another machine, or a device namespace path.
            Prefix::UNC(..) | Prefix::VerbatimUNC(..) | Prefix::Verbatim(_) | Prefix::DeviceNS(_) => {
                return Reliability::NotConfirmed;
            }
        },
        // A path without a drive is on the drive of the current directory.
        _ => None,
    };
    let root: Option<Vec<u16>> = root.map(|root| root.encode_wide().chain([0]).collect());
    let root = root.as_ref().map_or(std::ptr::null(), |root| root.as_ptr());
    // SAFETY: `root` is null or a NUL-terminated wide string that outlives the call.
    match unsafe { GetDriveTypeW(root) } {
        DRIVE_REMOTE | DRIVE_UNKNOWN | DRIVE_NO_ROOT_DIR => Reliability::NotConfirmed,
        _ => Reliability::KnownLocal,
    }
}
