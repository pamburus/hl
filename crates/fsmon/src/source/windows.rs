use std::{
    fs::{self, File, Metadata},
    io::{self, Read},
    os::windows::io::AsRawHandle,
    path::{Path, PathBuf},
    time::Duration,
};

use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_DISK, GetFileType};

use super::{Ctx, Input, Kind, Next, PipeReader, Stream};
use crate::stop::Stop;

// ---

/// Notifications may be delayed while a file is written to, so a watched file is also checked as a polled one is.
pub const DELAYED_NOTIFICATIONS: bool = true;

/// Returns how long a watched file is waited for: until its next recheck, or its next check as a polled file.
pub fn watch_timeout(recheck: Duration, interval: Duration) -> Duration {
    recheck.min(interval)
}

/// A file deleted while it is open, as it is here, may stay at its path until it is closed, and opening it fails with
/// an access error that tells nothing more. A file that another process holds without sharing it, such as a scanner,
/// fails with another error.
pub const DENIED_WHEN_DELETED: bool = true;

/// Whether `file` is a regular file, which the metadata of a pipe or a console tells too, so the type of the handle
/// tells it apart.
pub fn is_regular(file: &File, meta: &Metadata) -> bool {
    // SAFETY: `GetFileType` takes any handle, and the handle of `file` is open.
    meta.is_file() && unsafe { GetFileType(file.as_raw_handle()) } == FILE_TYPE_DISK
}

/// There are no FIFOs here, as a named pipe is not a file at a path that is followed.
pub fn is_fifo(_meta: &Metadata) -> bool {
    false
}

/// Devices are not told apart here, as they are read to their end as the only stream.
pub fn is_device(_meta: &Metadata) -> bool {
    false
}

/// Opens the file at `path` for reading, which never waits here.
pub fn open_nonblocking(path: &Path) -> io::Result<File> {
    fs::File::open(path)
}

/// Makes an input of `file` that is neither a directory nor a regular file, such as a pipe, read to its end as the
/// only stream.
pub fn add_special(ctx: &Ctx, _path: PathBuf, file: File, _meta: &Metadata) -> io::Result<Input> {
    let first = Stream(Kind::Pipe(PipeReader::new(ctx, Pipe::File(file))));
    Ok(ctx.input(Next::None, Some(first)))
}

// ---

/// A pipe read to its end.
pub enum Pipe {
    Stdin,
    /// A pipe given as an open file.
    File(File),
}

/// Reads `pipe`, or returns the end of the stream once stopped.
///
/// There is no wait for a pipe to be readable here, so a stop cancels the read instead.
pub fn read_pipe(stop: &Stop, pipe: &mut Pipe, buf: &mut [u8]) -> io::Result<usize> {
    stop.read(|| match pipe {
        Pipe::Stdin => io::stdin().lock().read(buf),
        Pipe::File(file) => file.read(buf),
    })
}
