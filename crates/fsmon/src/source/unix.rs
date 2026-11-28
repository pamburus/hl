use std::{
    fs::{self, File, Metadata},
    io::{self, Read},
    mem::ManuallyDrop,
    os::{
        fd::{AsFd, BorrowedFd, FromRawFd},
        unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use super::{Ctx, Input, Kind, Next, Opened, Reopened, Stream, lock, replaced};
use crate::{
    identity::FileId,
    stop::Stop,
    wait::{Wake, unix::wait_readable},
};

// ---

/// Notifications come as soon as a file changes, so a watched file is checked only when they say so.
pub const DELAYED_NOTIFICATIONS: bool = false;

/// Returns how long a watched file is waited for: until its next recheck, as notifications come at once.
pub fn watch_timeout(recheck: Duration, _interval: Duration) -> Duration {
    recheck
}

/// A file deleted while it is open is gone from its path at once, so a denied access tells nothing about it.
pub const DENIED_WHEN_DELETED: bool = false;

pub fn is_regular(_file: &File, meta: &Metadata) -> bool {
    meta.is_file()
}

pub fn is_fifo(meta: &Metadata) -> bool {
    meta.file_type().is_fifo()
}

/// Whether `meta` is of a character device, such as a terminal or `/dev/null`, which is followed as `tail -F` follows
/// it.
pub fn is_device(meta: &Metadata) -> bool {
    meta.file_type().is_char_device()
}

/// Opens the file at `path` for reading, without waiting for a writer if it is a FIFO.
///
/// The reads of a regular file are not changed by it, as they never wait for data. A terminal opened by it does not
/// become the controlling terminal of the process.
pub fn open_nonblocking(path: &Path) -> io::Result<File> {
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(path)
}

/// Makes an input of `file` at `path` that is neither a directory nor a regular file.
///
/// A FIFO is read from its next writer, and a character device as it is read. Any other special file, such as a block
/// device, cannot be followed, so a file is waited for to take its place, as with `tail -F`.
pub fn add_special(ctx: &Ctx, path: PathBuf, file: File, meta: &Metadata) -> io::Result<Input> {
    if is_device(meta) {
        let first = ctx.device(&path, file)?;
        return Ok(ctx.input(Next::Path(path), Some(first)));
    }
    if !is_fifo(meta) {
        return ctx.wait_for(path, meta);
    }
    let next = after_fifo(path, &file)?;
    Ok(ctx.fifo_input(next, file))
}

// ---

impl Ctx {
    /// Makes an input whose first stream is of the FIFO `file`.
    pub(super) fn fifo_input(&self, next: Next, file: File) -> Input {
        let reopened = Reopened::default();
        let path = match &next {
            Next::Path(path) => Some((path.clone(), reopened.clone())),
            Next::None => None,
        };
        Input {
            ctx: self.clone(),
            first: Some(self.fifo(file, path)),
            next,
            reopened,
        }
    }

    /// Makes a stream of the FIFO `file`, which opens the FIFO at the path that comes with
    /// `reopen` again for its next writer once it ends.
    pub(super) fn fifo(&self, file: File, reopen: Option<(PathBuf, Reopened)>) -> Stream {
        Stream(Kind::Fifo(FifoReader::new(self, file, reopen)))
    }

    /// Makes a stream of the character device `file` at `path`, which ends once the path refers to another file.
    pub(super) fn device(&self, path: &Path, file: File) -> io::Result<Stream> {
        Ok(Stream(Kind::Device(DeviceReader::new(self, path, file)?)))
    }
}

/// Returns what comes after the stream of the FIFO `file` at `path`: the stream of the next writer of a named FIFO,
/// and nothing for a pipe that has no name, such as one given by process substitution as a path in `/dev/fd`, as
/// opening that path again gives the same pipe, whose writer is gone for good.
pub fn after_fifo(path: PathBuf, file: &File) -> io::Result<Next> {
    // A pipe has no links on macOS and the BSDs, and one link on Linux, where it is on a file system of its own.
    #[cfg(target_os = "linux")]
    let unnamed = {
        use std::os::fd::AsRawFd;
        const PIPEFS_MAGIC: u32 = 0x5049_5045;
        // SAFETY: an all-zero `statfs` is a valid value of a plain C struct.
        let mut buf: libc::statfs = unsafe { std::mem::zeroed() };
        // SAFETY: the descriptor is open, and `buf` is a valid `statfs` to fill.
        if unsafe { libc::fstatfs(file.as_raw_fd(), &mut buf) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // The field is signed and of a different width on some targets, and the magic number is 32-bit.
        #[allow(clippy::unnecessary_cast)]
        let magic = buf.f_type as u32;
        magic == PIPEFS_MAGIC
    };
    #[cfg(not(target_os = "linux"))]
    let unnamed = {
        use std::os::unix::fs::MetadataExt;
        file.metadata()?.nlink() == 0
    };

    Ok(if unnamed { Next::None } else { Next::Path(path) })
}

// ---

/// Reads a FIFO from its next writer until it has no writer left, so writers that overlap write to one stream.
pub struct FifoReader {
    ctx: Ctx,
    file: File,
    /// The path of the FIFO, to open it again for its next writer, and where to put it.
    reopen: Option<(PathBuf, Reopened)>,
    /// Whether the stream ended, after which nothing more is read, as what the next writer writes is for the next
    /// stream, which shares the buffer of the FIFO.
    ended: bool,
}

impl FifoReader {
    /// Makes a reader of a FIFO, which may be blocking or not.
    fn new(ctx: &Ctx, file: File, reopen: Option<(PathBuf, Reopened)>) -> Self {
        Self {
            ctx: ctx.clone(),
            file,
            reopen,
            ended: false,
        }
    }

    /// Opens the FIFO again for its next writer while it is still open, as a writer that opens it in between would get
    /// an error, or lose what it wrote, once it is no longer open by any reader.
    fn reopen_for_next_writer(&mut self) {
        if let Some((path, reopened)) = self.reopen.take() {
            // Otherwise the input opens the path again for its next stream.
            if let Ok(Opened::Fifo(file)) = self.ctx.open(&path) {
                *lock(&reopened) = Some(file);
            }
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.ended || self.ctx.stop.is_stopped() {
                return Ok(0);
            }
            // The blocking mode of the FIFO is left as it was opened, as it may be shared with other processes, such as
            // the standard input given by its path, so a read waits for it to be readable first, blocking or not. It is
            // not readable before its first writer opens it, and is readable once its last writer closes it, when it
            // reads as empty.
            let mask = wait_readable(&[self.file.as_fd(), self.ctx.stop.fd()], None)?;
            if mask & 0b10 != 0 {
                return Ok(0);
            }
            if mask & 0b01 == 0 {
                continue;
            }
            match self.file.read(buf) {
                Ok(0) => {
                    self.ended = true;
                    self.reopen_for_next_writer();
                    return Ok(0);
                }
                Ok(n) => return Ok(n),
                // A FIFO opened by its path, here or by the consumer, is not blocking, and another reader may have read what
                // was there.
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }
    }
}

// ---

/// Reads a character device, such as a terminal or `/dev/null`, as `tail -F` reads it.
///
/// What can be read from it without waiting is read at first, and again each time its modification time or mode
/// changes, which is checked once per poll interval. So what is typed into a terminal, which changes neither, is left
/// unread, as with `tail -F`. A device may have no end, such as `/dev/null`, whose reads return nothing, so the stream
/// ends only once the path refers to another file, which is checked once per recheck cadence, as for a regular file
/// with no notifications.
pub struct DeviceReader {
    ctx: Ctx,
    path: PathBuf,
    file: File,
    id: FileId,
    /// The modification time and mode of the device as last seen.
    stamp: Stamp,
    /// Whether what can be read without waiting is to be read.
    readable: bool,
    /// When the path was last checked for another file.
    checked: Instant,
    ended: bool,
}

/// The modification time, in seconds and nanoseconds, and mode of a device.
type Stamp = (i64, i64, u32);

fn stamp(meta: &Metadata) -> Stamp {
    (meta.mtime(), meta.mtime_nsec(), meta.mode())
}

impl DeviceReader {
    fn new(ctx: &Ctx, path: &Path, file: File) -> io::Result<Self> {
        Ok(Self {
            ctx: ctx.clone(),
            path: path.to_owned(),
            id: FileId::of_file(&file)?,
            stamp: stamp(&file.metadata()?),
            file,
            readable: true,
            checked: Instant::now(),
            ended: false,
        })
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.ended || self.ctx.stop.is_stopped() {
                return Ok(0);
            }
            if self.readable {
                match self.read_now(buf)? {
                    0 => self.readable = false,
                    n => return Ok(n),
                }
            }
            if self.ctx.stop.sleep(self.ctx.options.poll_interval)? == Wake::Stopped {
                return Ok(0);
            }
            if let Ok(meta) = self.file.metadata() {
                let stamp = stamp(&meta);
                if stamp != self.stamp {
                    self.stamp = stamp;
                    self.readable = true;
                }
            }
            if self.checked.elapsed() >= self.ctx.options.recheck_cadence {
                self.checked = Instant::now();
                if replaced(FileId::of_path(&self.path), self.id) {
                    log::info!("fsmon: {} was replaced or removed", self.path.display());
                    self.ended = true;
                }
            }
        }
    }

    /// Reads what can be read without waiting, which is nothing at the end of the device, as of `/dev/null`.
    ///
    /// The blocking mode of the device is left as it was opened, as it may be shared with other processes, so it is
    /// read only once it is readable.
    fn read_now(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if wait_readable(&[self.file.as_fd()], Some(Duration::ZERO))? & 1 == 0 {
                return Ok(0);
            }
            match self.file.read(buf) {
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => return Ok(0),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                result => return result,
            }
        }
    }
}

// ---

/// A pipe read to its end, which is only the standard input, as a pipe given as an open file is a FIFO.
pub enum Pipe {
    Stdin,
}

/// Reads `pipe` once it is readable, or returns the end of the stream once stopped.
///
/// The pipe is left blocking, as it may be shared with other processes, so a read waits for data first.
pub fn read_pipe(stop: &Stop, pipe: &mut Pipe, buf: &mut [u8]) -> io::Result<usize> {
    let Pipe::Stdin = pipe;
    // SAFETY: the standard input stays open for the life of the process.
    let fd = unsafe { BorrowedFd::borrow_raw(0) };
    loop {
        let mask = wait_readable(&[fd, stop.fd()], None)?;
        if mask & 0b10 != 0 {
            return Ok(0);
        }
        if mask & 0b01 != 0 {
            return read_stdin(buf);
        }
    }
}

/// Reads the standard input past the buffer of the standard library, which a wait for readiness would not see.
fn read_stdin(buf: &mut [u8]) -> io::Result<usize> {
    // SAFETY: the standard input stays open for the life of the process, and is not closed here.
    let mut file = ManuallyDrop::new(unsafe { File::from_raw_fd(0) });
    file.read(buf)
}
