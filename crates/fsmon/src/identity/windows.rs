use std::{fs::File, io, mem, os::windows::io::AsRawHandle, path::Path};

use windows_sys::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_ID_INFO, FileIdInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
    },
};

// ---

/// The serial number of the volume and the file identifier within it.
pub type Id = (u64, u128);

pub fn of_file(file: &File) -> io::Result<Id> {
    let handle = file.as_raw_handle() as HANDLE;
    // SAFETY: an all-zero `FILE_ID_INFO` is a valid value of a plain C struct.
    let mut info: FILE_ID_INFO = unsafe { mem::zeroed() };
    // SAFETY: `handle` is an open file and `info` is a buffer of the size given.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            (&raw mut info).cast(),
            mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if ok != 0 {
        return Ok((info.VolumeSerialNumber, u128::from_ne_bytes(info.FileId.Identifier)));
    }
    // The 128-bit identifier is not available on some file systems and older versions of Windows.
    // SAFETY: an all-zero `BY_HANDLE_FILE_INFORMATION` is a valid value of a plain C struct.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { mem::zeroed() };
    // SAFETY: `handle` is an open file and `info` is a valid struct to fill.
    if unsafe { GetFileInformationByHandle(handle, &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok((u64::from(info.dwVolumeSerialNumber), u128::from(index)))
}

pub fn of_path(path: &Path) -> io::Result<Id> {
    // Files are opened sharing deletion, so a file opened here can still be renamed or deleted.
    of_file(&File::open(path)?)
}
