use std::fs::File;
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_ID_INFO, FileIdInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
};

// ---

/// (volume serial number, 128-bit file id). Falls back to a 64-bit file index
/// combined with volume serial when `GetFileInformationByHandleEx`/
/// `FILE_ID_INFO` is not available.
pub type Inner = (u32, u128);

pub fn from_file(file: &File) -> std::io::Result<Inner> {
    let handle = file.as_raw_handle() as HANDLE;
    if let Some(id) = try_file_id_info(handle) {
        return Ok(id);
    }
    fallback_from_handle(handle)
}

pub fn from_path(path: &Path) -> std::io::Result<Inner> {
    let file = std::fs::File::open(path)?;
    from_file(&file)
}

fn try_file_id_info(handle: HANDLE) -> Option<Inner> {
    let mut info = FILE_ID_INFO {
        VolumeSerialNumber: 0,
        FileId: windows_sys::Win32::Storage::FileSystem::FILE_ID_128 { Identifier: [0u8; 16] },
    };
    let ok = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if ok != 0 {
        let id_bytes = info.FileId.Identifier;
        let file_id = u128::from_ne_bytes(id_bytes);
        Some((info.VolumeSerialNumber as u32, file_id))
    } else {
        None
    }
}

fn fallback_from_handle(handle: HANDLE) -> std::io::Result<Inner> {
    let mut info = BY_HANDLE_FILE_INFORMATION {
        dwFileAttributes: 0,
        ftCreationTime: unsafe { std::mem::zeroed() },
        ftLastAccessTime: unsafe { std::mem::zeroed() },
        ftLastWriteTime: unsafe { std::mem::zeroed() },
        dwVolumeSerialNumber: 0,
        nFileSizeHigh: 0,
        nFileSizeLow: 0,
        nNumberOfLinks: 0,
        nFileIndexHigh: 0,
        nFileIndexLow: 0,
    };
    let ok = unsafe { GetFileInformationByHandle(handle, &mut info) };
    if ok != 0 {
        let file_index = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
        Ok((info.dwVolumeSerialNumber, file_index as u128))
    } else {
        Err(std::io::Error::last_os_error())
    }
}
