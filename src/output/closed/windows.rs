use std::{mem, ptr, thread, time::Duration};

use windows_sys::{
    Wdk::Storage::FileSystem::{
        FILE_PIPE_CLOSING_STATE, FILE_PIPE_LOCAL_INFORMATION, FilePipeLocalInformation, NtQueryInformationFile,
    },
    Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{FILE_TYPE_PIPE, GetFileType},
        System::{
            Console::{GetStdHandle, STD_OUTPUT_HANDLE},
            IO::IO_STATUS_BLOCK,
        },
    },
};

// ---

/// How often the state of the pipe is checked, well within the second in which a closed output is to be found.
const INTERVAL: Duration = Duration::from_millis(100);

fn handle() -> HANDLE {
    // SAFETY: `GetStdHandle` takes no pointers.
    unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }
}

/// Whether the standard output is a pipe whose state can be checked, which includes one already closed by its
/// reader, so that this is noticed too.
pub fn watchable() -> bool {
    // SAFETY: `GetFileType` takes any handle, and fails on one that is not valid.
    let kind = unsafe { GetFileType(handle()) };
    kind == FILE_TYPE_PIPE && closing().is_some()
}

/// Returns true once the standard output is closed by its reader, or false if that cannot be checked.
///
/// Windows does not report that the reader of a pipe has closed it until something is written to it, so the
/// state of the pipe is checked periodically instead.
pub fn wait_closed() -> bool {
    loop {
        match closing() {
            Some(false) => thread::sleep(INTERVAL),
            Some(true) => return true,
            None => return false,
        }
    }
}

/// Whether the other end of the pipe is closed, or `None` if that cannot be checked.
fn closing() -> Option<bool> {
    // SAFETY: both structures are plain data, so all zeros is a valid value.
    let mut status: IO_STATUS_BLOCK = unsafe { mem::zeroed() };
    let mut info: FILE_PIPE_LOCAL_INFORMATION = unsafe { mem::zeroed() };
    // SAFETY: `info` is a buffer of the given size for the information class asked for, and `status` receives the
    // status of the call.
    let result = unsafe {
        NtQueryInformationFile(
            handle(),
            &mut status,
            ptr::from_mut(&mut info).cast(),
            mem::size_of_val(&info) as u32,
            FilePipeLocalInformation,
        )
    };
    (result >= 0).then_some(info.NamedPipeState == FILE_PIPE_CLOSING_STATE)
}
