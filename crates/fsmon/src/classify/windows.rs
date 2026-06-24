use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{DRIVE_NO_ROOT_DIR, DRIVE_REMOTE, DRIVE_UNKNOWN, GetDriveTypeW};

use super::Reliability;

// ---

pub fn classify(path: &Path) -> Reliability {
    // UNC paths (\\server\share\...) are network paths.
    if is_unc(path) {
        return Reliability::NotConfirmed;
    }

    let root = drive_root(path);
    let root_wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();

    let drive_type = unsafe { GetDriveTypeW(root_wide.as_ptr()) };
    match drive_type {
        DRIVE_REMOTE | DRIVE_UNKNOWN | DRIVE_NO_ROOT_DIR => Reliability::NotConfirmed,
        _ => Reliability::KnownLocal,
    }
}

fn is_unc(path: &Path) -> bool {
    let s = path.to_str().unwrap_or("");
    s.starts_with("\\\\") || s.starts_with("//")
}

fn drive_root(path: &Path) -> String {
    let s = path.to_str().unwrap_or("");
    if s.len() >= 3 && s.chars().nth(1) == Some(':') {
        s[..3].to_string()
    } else {
        "\\".to_string()
    }
}
