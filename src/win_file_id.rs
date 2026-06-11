// Windows counterpart to Unix inode+device identity.  Standard metadata does
// not expose a stable file identity on Windows, so we call
// `GetFileInformationByHandle` directly to obtain the volume serial number
// and file index, which together uniquely identify a file across renames.

use std::{fs, mem, os::windows::fs::OpenOptionsExt, os::windows::io::AsRawHandle, path::Path};

const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const FILE_SHARE_DELETE: u32 = 0x00000004;

#[repr(C)]
struct ByHandleFileInformation {
    dw_file_attributes: u32,
    ft_creation_time: [u32; 2],
    ft_last_access_time: [u32; 2],
    ft_last_write_time: [u32; 2],
    dw_volume_serial_number: u32,
    n_file_size_high: u32,
    n_file_size_low: u32,
    n_number_of_links: u32,
    n_file_index_high: u32,
    n_file_index_low: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileInformationByHandle(
        h_file: *mut core::ffi::c_void,
        lp_file_information: *mut ByHandleFileInformation,
    ) -> i32;
}

#[derive(Clone, PartialEq)]
pub struct FileId {
    file_index: u64,
    volume_serial_number: u32,
}

pub struct FileInfo {
    pub id: FileId,
    pub size: u64,
}

// FILE_SHARE_DELETE is required to open files that are currently being
// renamed or deleted (the common state during log rotation).
pub fn open_shared(path: &Path) -> Option<fs::File> {
    fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(path)
        .ok()
}

pub fn query(file: &fs::File) -> Option<FileInfo> {
    let mut info: ByHandleFileInformation = unsafe { mem::zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } != 0 {
        Some(FileInfo {
            id: FileId {
                file_index: ((info.n_file_index_high as u64) << 32) | info.n_file_index_low as u64,
                volume_serial_number: info.dw_volume_serial_number,
            },
            size: ((info.n_file_size_high as u64) << 32) | info.n_file_size_low as u64,
        })
    } else {
        None
    }
}
