//! Inputs and the byte streams they deliver.

use std::{
    fs::{self, File, Metadata},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::{
    classify::{Reliability, classify},
    identity::FileId,
    interval::{FLOOR, IntervalEstimator},
    options::{FallbackPolicy, FollowOptions},
    stop::{Stop, Stopper},
    wait::{Wake, Watch, dir_of},
};

// ---

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as imp;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

use imp::{Pipe, is_device, is_fifo, is_regular, open_nonblocking};

// ---

/// Follows files by name, as `tail -F` does.
///
/// Each input added to it delivers a sequence of byte streams, one per file that appears at its path,
/// so that rotation, truncation and replacement each start a new stream, read from the start of the file.
/// The consumer tells entries apart, and may finish an unterminated entry once its stream ends.
pub struct Follow {
    ctx: Ctx,
}

impl Follow {
    /// Creates a follow with the given options.
    pub fn new(mut options: FollowOptions) -> io::Result<Self> {
        // A shorter interval would make the checks spin.
        options.poll_interval = options.poll_interval.max(FLOOR);
        options.recheck_cadence = options.recheck_cadence.max(FLOOR);
        Ok(Self {
            ctx: Ctx {
                options: Arc::new(options),
                stop: Stop::new()?,
            },
        })
    }

    /// Returns a handle that stops the follow from any thread.
    pub fn stopper(&self) -> Stopper {
        Stopper(self.ctx.stop.clone())
    }

    /// Adds the file at `path`, whose first stream starts at the end of the file.
    ///
    /// A FIFO is read from its next writer, and a character device as it is read, as `tail -F` reads them. A path that
    /// is missing or cannot be opened is waited for if the options allow it, and its first stream then starts at the
    /// start of the file. So is a directory or any other special file that cannot be followed, until a file takes its
    /// place, as with `tail -F`.
    pub fn add_path(&self, path: impl Into<PathBuf>) -> io::Result<Input> {
        self.add(path.into(), SeekFrom::End(0))
    }

    /// Adds the file at `path`, whose first stream starts at the start of the file, as [`add_path`](Self::add_path)
    /// does for a file that appears later.
    ///
    /// It is meant for a path found missing or unreadable before, whose file may have appeared since, and is then read
    /// whole, as one that appears later would be.
    pub fn add_path_from_start(&self, path: impl Into<PathBuf>) -> io::Result<Input> {
        self.add(path.into(), SeekFrom::Start(0))
    }

    fn add(&self, path: PathBuf, start: SeekFrom) -> io::Result<Input> {
        let first = match self.ctx.open(&path) {
            Ok(Opened::File(mut file)) => {
                let pos = file.seek(start)?;
                Some(self.ctx.regular(&path, file, pos)?)
            }
            #[cfg(unix)]
            Ok(Opened::Fifo(file)) => {
                let next = unix::after_fifo(path, &file)?;
                return Ok(self.ctx.fifo_input(next, file));
            }
            #[cfg(unix)]
            Ok(Opened::Device(file)) => Some(self.ctx.device(&path, file)?),
            Ok(Opened::Special(meta)) => return self.ctx.wait_for(path, &meta),
            Err(err) if !self.ctx.options.retry_missing => return Err(err),
            // Its first stream waits for it, which reports why it cannot be opened.
            Err(_) => None,
        };
        Ok(self.ctx.input(Next::Path(path), first))
    }

    /// Adds the file at `path` that is already open as `file`, whose first stream starts at the current position of `file`.
    ///
    /// Any later stream is of the file at `path`, as with [`add_path`](Self::add_path). Once the path refers to
    /// another file, the rest of `file` is read before the stream ends. A pipe that has no name is read to its end as
    /// the only stream. A character device is followed as [`add_path`](Self::add_path) follows it on Unix, and other
    /// special files are waited for as it waits for them. Elsewhere, special files are read to their end as the only
    /// stream. A directory is waited for, as with [`add_path`](Self::add_path).
    pub fn add_file(&self, path: impl Into<PathBuf>, mut file: File) -> io::Result<Input> {
        let path = path.into();
        let meta = file.metadata()?;
        if meta.is_dir() {
            return self.ctx.wait_for(path, &meta);
        }
        if is_regular(&file, &meta) {
            let pos = file.stream_position()?;
            let first = self.ctx.regular(&path, file, pos)?;
            return Ok(self.ctx.input(Next::Path(path), Some(first)));
        }
        imp::add_special(&self.ctx, path, file, &meta)
    }

    /// Adds the standard input, which delivers a single stream.
    pub fn add_stdin(&self) -> Input {
        let first = Stream(Kind::Pipe(PipeReader::new(&self.ctx, Pipe::Stdin)));
        self.ctx.input(Next::None, Some(first))
    }
}

// ---

/// A followed input, which delivers a stream per file that appears at its path.
pub struct Input {
    ctx: Ctx,
    next: Next,
    first: Option<Stream>,
    /// The FIFO at the path, opened again for its next writer before the stream of the last one ended.
    reopened: Reopened,
}

impl Input {
    /// Returns the next stream of the input, waiting for a file to appear at its path if needed.
    ///
    /// Returns `None` once the follow is stopped, or once no more streams can come, as for the standard input,
    /// or for a path that is missing when the options do not allow waiting for it.
    pub fn next_stream(&mut self) -> io::Result<Option<Stream>> {
        if self.ctx.stop.is_stopped() {
            return Ok(None);
        }
        if let Some(first) = self.first.take() {
            return Ok(Some(first));
        }
        match std::mem::replace(&mut self.next, Next::None) {
            Next::None => Ok(None),
            Next::Path(path) => {
                #[cfg(unix)]
                if let Some(file) = lock(&self.reopened).take() {
                    let stream = self.ctx.fifo(file, Some((path.clone(), self.reopened.clone())));
                    self.next = Next::Path(path);
                    return Ok(Some(stream));
                }
                let stream = self.ctx.reopen(&path, &self.reopened)?;
                if stream.is_some() {
                    self.next = Next::Path(path);
                }
                Ok(stream)
            }
        }
    }
}

/// Where the stream of a FIFO puts the FIFO opened again for its next writer, for the input to start its next stream.
type Reopened = Arc<Mutex<Option<File>>>;

#[cfg(unix)]
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// What comes after the current stream of an input.
enum Next {
    /// The file at the path, from its start.
    Path(PathBuf),
    /// Nothing.
    None,
}

// ---

/// A byte stream of a single file, which ends once the file is truncated, replaced or removed, or the follow stops.
///
/// A read waits until there is data to return, so it returns zero only at the end of the stream.
pub struct Stream(Kind);

enum Kind {
    File(FileReader),
    #[cfg(unix)]
    Fifo(unix::FifoReader),
    #[cfg(unix)]
    Device(unix::DeviceReader),
    Pipe(PipeReader),
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        match &mut self.0 {
            Kind::File(reader) => reader.read(buf),
            #[cfg(unix)]
            Kind::Fifo(reader) => reader.read(buf),
            #[cfg(unix)]
            Kind::Device(reader) => reader.read(buf),
            Kind::Pipe(reader) => reader.read(buf),
        }
    }
}

// ---

/// The state shared by a follow and its inputs.
#[derive(Clone)]
struct Ctx {
    options: Arc<FollowOptions>,
    stop: Arc<Stop>,
}

impl Ctx {
    fn input(&self, next: Next, first: Option<Stream>) -> Input {
        Input {
            ctx: self.clone(),
            next,
            first,
            reopened: Reopened::default(),
        }
    }

    /// Makes an input of `path`, which holds a file that cannot be followed, such as a directory, that waits for a file
    /// to take its place, as with `tail -F`, or fails if the options do not allow waiting.
    fn wait_for(&self, path: PathBuf, meta: &Metadata) -> io::Result<Input> {
        if !self.options.retry_missing {
            return Err(unsupported(&path, meta));
        }
        // Its first stream waits for it, which reports why it cannot be followed.
        Ok(self.input(Next::Path(path), None))
    }

    /// Opens the file at `path` without blocking, which a FIFO with no writer would do otherwise.
    fn open(&self, path: &Path) -> io::Result<Opened> {
        // Other kinds of files, such as directories and block devices, are not opened, unless one replaces the path
        // between the check and the open, when it is told apart by the kind of the file opened.
        let meta = fs::metadata(path)?;
        if !meta.is_file() && !is_fifo(&meta) && !is_device(&meta) {
            return Ok(Opened::Special(meta));
        }
        // The path may be replaced by another kind of file before it is opened, so the kind of the file opened is
        // checked again, and the open does not block, which it would for a FIFO with no writer.
        let file = open_nonblocking(path)?;
        let meta = file.metadata()?;
        if is_regular(&file, &meta) {
            return Ok(Opened::File(file));
        }
        #[cfg(unix)]
        if is_fifo(&meta) {
            return Ok(Opened::Fifo(file));
        }
        #[cfg(unix)]
        if is_device(&meta) {
            return Ok(Opened::Device(file));
        }
        Ok(Opened::Special(meta))
    }

    /// Waits for a file to appear at `path` and returns its stream from the start.
    ///
    /// Returns `None` once stopped, or once no file can be opened when the options do not allow waiting for one.
    #[cfg_attr(not(unix), allow(unused_variables))]
    fn reopen(&self, path: &Path, reopened: &Reopened) -> io::Result<Option<Stream>> {
        let mut watch: Option<Watch> = None;
        let mut logged = false;
        // Decided once, as it is logged when the path is polled.
        let mut native = self.native(path);
        loop {
            if self.stop.is_stopped() {
                return Ok(None);
            }
            // Set up before the file is looked for, so that a file created right after is not missed.
            if watch.is_none() && native {
                watch = Watch::new(path, None, &self.stop)
                    .inspect_err(|err| {
                        if !logged {
                            log::debug!("fsmon: polling for {}: {err}", path.display());
                        }
                    })
                    .ok();
            }
            let reason = match self.open(path) {
                Ok(Opened::File(file)) => {
                    log::info!("fsmon: following {} from its start", path.display());
                    return self.regular(path, file, 0).map(Some);
                }
                #[cfg(unix)]
                Ok(Opened::Fifo(file)) => {
                    return Ok(Some(self.fifo(file, Some((path.to_owned(), reopened.clone())))));
                }
                #[cfg(unix)]
                Ok(Opened::Device(file)) => {
                    log::info!("fsmon: following {}", path.display());
                    return self.device(path, file).map(Some);
                }
                Ok(Opened::Special(meta)) => unsupported(path, &meta),
                Err(err) => err,
            };
            if !self.options.retry_missing {
                log::warn!("fsmon: stopped following {}: {reason}", path.display());
                return Ok(None);
            }
            if !logged {
                log::warn!("fsmon: waiting for {}: {reason}", path.display());
                logged = true;
            }
            // A file that cannot be opened may become readable with no notification, as on macOS, where it cannot
            // be watched itself, so it is checked as often as a polled path, as `tail -F` does without notifications.
            let timeout = match reason.kind() {
                io::ErrorKind::NotFound => self.options.recheck_cadence,
                _ => self.options.recheck_cadence.min(self.options.poll_interval),
            };
            let wake = match &mut watch {
                Some(w) => match w.wait(Some(timeout)) {
                    Ok(wake) => wake,
                    // A watch that fails is not set up again, as it could fail at once each time.
                    Err(err) => {
                        log::debug!("fsmon: polling for {}: {err}", path.display());
                        native = false;
                        Wake::Ready
                    }
                },
                None => self.stop.sleep(self.options.poll_interval)?,
            };
            match wake {
                Wake::Stopped => return Ok(None),
                // The directory itself may have been replaced, which ends its watch, so it is set up again.
                Wake::Ready => watch = None,
                Wake::Timeout => {}
            }
        }
    }

    /// Tells whether changes to `path` may be waited for with native notifications, logging the decision.
    fn native(&self, path: &Path) -> bool {
        match self.options.fallback_policy {
            FallbackPolicy::Optimistic => true,
            FallbackPolicy::Poll => {
                log::debug!("fsmon: polling {}: every path is polled", path.display());
                false
            }
            FallbackPolicy::Conservative => {
                let target = fs::canonicalize(path)
                    .ok()
                    .or_else(|| fs::canonicalize(dir_of(path)).ok());
                let native = target.is_some_and(|target| classify(&target) == Reliability::KnownLocal);
                if !native {
                    log::debug!(
                        "fsmon: polling {}: its file system is not known to be local",
                        path.display()
                    );
                }
                native
            }
        }
    }

    fn regular(&self, path: &Path, file: File, pos: u64) -> io::Result<Stream> {
        Ok(Stream(Kind::File(FileReader::new(self, path, file, pos)?)))
    }
}

/// A file opened at a path.
enum Opened {
    File(File),
    #[cfg(unix)]
    Fifo(File),
    /// A character device.
    #[cfg(unix)]
    Device(File),
    /// A file that cannot be followed, such as a directory, which is not opened.
    Special(Metadata),
}

fn unsupported(path: &Path, meta: &Metadata) -> io::Error {
    let what = if meta.is_dir() { "a directory" } else { "a special file" };
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{} is {what}, which cannot be followed", path.display()),
    )
}

// ---

/// Reads a regular file until it is truncated, replaced or removed.
struct FileReader {
    ctx: Ctx,
    path: PathBuf,
    file: File,
    id: FileId,
    pos: u64,
    watch: Option<Watch>,
    estimator: IntervalEstimator,
    /// Whether the last read came after a wait.
    waited: bool,
    /// Whether the file was found longer than read since the last wait, which is read again only once per wait.
    grown: bool,
    /// Whether the path refers to another file, so the rest of this one is read before the stream ends.
    ending: bool,
    ended: bool,
}

impl FileReader {
    fn new(ctx: &Ctx, path: &Path, file: File, pos: u64) -> io::Result<Self> {
        let id = FileId::of_file(&file)?;
        let watch = if ctx.native(path) {
            Watch::new(path, Some(&file), &ctx.stop)
                .inspect(|_| log::debug!("fsmon: watching {} with native notifications", path.display()))
                .inspect_err(|err| log::debug!("fsmon: polling {}: {err}", path.display()))
                .ok()
        } else {
            None
        };
        Ok(Self {
            ctx: ctx.clone(),
            path: path.to_owned(),
            file,
            id,
            pos,
            watch,
            estimator: IntervalEstimator::new(ctx.options.poll_interval, Instant::now()),
            waited: false,
            grown: false,
            ending: false,
            ended: false,
        })
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.ended || self.ctx.stop.is_stopped() {
                return Ok(0);
            }
            match self.file.read(buf) {
                Ok(0) => {}
                Ok(n) => {
                    self.pos += n as u64;
                    if std::mem::take(&mut self.waited) && self.adaptive() {
                        self.estimator.on_change(Instant::now());
                    }
                    return Ok(n);
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
            if self.ending {
                self.ended = true;
                return Ok(0);
            }
            match self.status() {
                Status::Truncated => {
                    // A file found shorter than just read from it, with no wait in between, such as one in procfs whose
                    // size stays zero while it has content, is read again only after the poll interval, rather than
                    // again and again with no pause. This is one of the differences from `tail -F` that the spec lists.
                    if !self.waited {
                        self.ctx.stop.sleep(self.ctx.options.poll_interval)?;
                    }
                    log::info!("fsmon: {} was truncated", self.path.display());
                    self.ended = true;
                    return Ok(0);
                }
                Status::Grown => {
                    self.grown = true;
                    continue;
                }
                Status::Replaced => {
                    log::info!("fsmon: {} was replaced or removed", self.path.display());
                    // Whatever was written before the replacement is read before the stream ends.
                    self.ending = true;
                    continue;
                }
                Status::Same => {}
            }
            self.wait()?;
        }
    }

    /// Waits for a change, the next check, or a stop.
    fn wait(&mut self) -> io::Result<()> {
        let wake = match &mut self.watch {
            Some(watch) => {
                let timeout = self.ctx.options.recheck_cadence;
                let timeout = imp::watch_timeout(timeout, self.estimator.interval());
                match watch.wait(Some(timeout)) {
                    Ok(wake) => wake,
                    Err(err) => {
                        log::debug!("fsmon: polling {}: {err}", self.path.display());
                        self.watch = None;
                        Wake::Ready
                    }
                }
            }
            None => self.ctx.stop.sleep(self.estimator.interval())?,
        };
        if wake == Wake::Timeout && self.adaptive() {
            self.estimator.on_no_change(Instant::now());
        }
        self.waited = true;
        self.grown = false;
        Ok(())
    }

    /// Tells whether the interval between checks adapts to the changes.
    fn adaptive(&self) -> bool {
        self.watch.is_none() || imp::DELAYED_NOTIFICATIONS
    }

    fn status(&self) -> Status {
        if let Ok(meta) = self.file.metadata() {
            if meta.len() < self.pos {
                return Status::Truncated;
            }
            // Once the file is read again to its end, a size that is still larger tells nothing more, as of a file in
            // sysfs, whose size is larger than its content, so the path is checked for another file then.
            if meta.len() > self.pos && !self.grown {
                return Status::Grown;
            }
        }
        if replaced(FileId::of_path(&self.path), self.id) {
            Status::Replaced
        } else {
            Status::Same
        }
    }
}

/// Tells whether a path holds another file than the one with `id`, or none, from what was `found` at it.
///
/// A check that fails for another reason, such as a stale handle on a network file system, may fail only briefly,
/// so it is taken as the same file and made again later. Taken as a replacement, it would have the same file read
/// again from its start.
///
/// A denied access is taken as a replacement too where that is what a file deleted while it is open gives.
fn replaced(found: io::Result<FileId>, id: FileId) -> bool {
    match found {
        Ok(found) => found != id,
        Err(err) => {
            matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory)
                || (imp::DENIED_WHEN_DELETED && err.kind() == io::ErrorKind::PermissionDenied)
        }
    }
}

/// What a check of a regular file found once all of it was read.
enum Status {
    /// The file is shorter than read, so it was truncated.
    Truncated,
    /// The file is longer than read, so the read may have raced with a write. Found only once until the file is read
    /// to its end again, as a file whose size is larger than its content, as in sysfs, would be found so forever.
    Grown,
    /// The path refers to another file, or none.
    Replaced,
    Same,
}

// ---

/// Reads a pipe to its end.
struct PipeReader {
    ctx: Ctx,
    pipe: Pipe,
}

impl PipeReader {
    fn new(ctx: &Ctx, pipe: Pipe) -> Self {
        Self { ctx: ctx.clone(), pipe }
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.ctx.stop.is_stopped() {
                return Ok(0);
            }
            match imp::read_pipe(&self.ctx.stop, &mut self.pipe, buf) {
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                result => return result,
            }
        }
    }
}

// ---

#[cfg(test)]
mod tests;
