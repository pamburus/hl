use std::collections::VecDeque;
use std::io::{self, Read};
use std::path::Path;
use std::sync::mpsc::RecvTimeoutError;

use crate::options::FollowOptions;
use crate::watch::Watcher;
use crate::{Error, Result, SourceId};

use source::FollowedSource;

// ---

pub mod source;

// ---

/// A run of bytes from one followed source.
pub struct Chunk {
    pub source: SourceId,
    pub bytes: Vec<u8>,
}

// ---

/// Follows one or more paths. The simple path is `follow(path)?.into_reader()`.
///
/// Internally: a background watcher thread delivers coalesced wake signals;
/// reconciliation and reading happen on the consumer's thread in
/// `next_chunk()` (backpressure — data waits on disk for regular files).
pub struct Follower {
    sources: Vec<FollowedSource>,
    watcher: Watcher,
    buf: Vec<u8>,
}

impl Follower {
    pub fn new<I>(paths: I, options: FollowOptions) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        let paths: Vec<std::path::PathBuf> = paths.into_iter().map(|p| p.as_ref().to_path_buf()).collect();

        let watcher = Watcher::new(paths.iter(), options.clone())?;

        let buf_size = options.read_buffer;
        let mut sources = Vec::with_capacity(paths.len());
        for path in paths {
            sources.push(FollowedSource::open(path, options.clone())?);
        }

        Ok(Self {
            sources,
            watcher,
            buf: vec![0u8; buf_size],
        })
    }

    /// Returns the next chunk of bytes from any followed source, blocking
    /// until data is available.
    ///
    /// Returns `Ok(None)` only when all sources have permanently ended
    /// (`retry_missing = false` and the path is gone).
    pub fn next_chunk(&mut self) -> Result<Option<Chunk>> {
        let tick = self.watcher.tick_interval();

        loop {
            // First, try to read from all sources without waiting.
            for idx in 0..self.sources.len() {
                match self.sources[idx].read_available(&mut self.buf) {
                    Ok(0) => {} // No data from this source yet.
                    Ok(n) => {
                        return Ok(Some(Chunk {
                            source: SourceId(idx),
                            bytes: self.buf[..n].to_vec(),
                        }));
                    }
                    Err(e) => return Err(e),
                }
            }

            // Check if all sources are permanently done.
            if !self.sources.is_empty() && self.sources.iter().all(|s| s.is_done()) {
                return Ok(None);
            }

            // Wait for the next wake signal (or tick).
            match self.watcher.recv_timeout(tick) {
                Ok(signal) => {
                    // A specific source was woken. Try to read from it.
                    let idx = signal.source_idx;
                    if idx < self.sources.len() {
                        match self.sources[idx].read_available(&mut self.buf) {
                            Ok(0) => {} // Nothing yet — loop to wait again.
                            Ok(n) => {
                                return Ok(Some(Chunk {
                                    source: SourceId(idx),
                                    bytes: self.buf[..n].to_vec(),
                                }));
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    // Recheck tick — reconcile all sources (FR-006, FR-016).
                    // The loop head will re-try all sources on the next iteration.
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "watch backend stopped",
                    )));
                }
            }
        }
    }

    /// Consumes this `Follower` and returns a `Read` implementation that
    /// yields appended bytes across rotation/truncation/deletion (FR-013).
    ///
    /// Only valid for single-source followers; for multi-source, use
    /// `next_chunk()`.
    pub fn into_reader(self) -> impl Read + Send {
        FollowerReader {
            follower: self,
            buffer: VecDeque::new(),
        }
    }
}

// ---

struct FollowerReader {
    follower: Follower,
    buffer: VecDeque<u8>,
}

impl Read for FollowerReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if !self.buffer.is_empty() {
                let n = buf.len().min(self.buffer.len());
                let slice: Vec<u8> = self.buffer.drain(..n).collect();
                buf[..n].copy_from_slice(&slice);
                return Ok(n);
            }

            match self.follower.next_chunk() {
                Ok(Some(chunk)) => {
                    self.buffer.extend(chunk.bytes.iter());
                }
                Ok(None) => return Ok(0), // All sources ended.
                Err(e) => return Err(io::Error::other(e)),
            }
        }
    }
}

// ---

/// Follow one path with default options. The simplest entry point.
///
/// ```rust,no_run
/// let mut reader = fsmon::follow("/var/log/app.log").unwrap().into_reader();
/// ```
pub fn follow(path: impl AsRef<Path>) -> Result<Follower> {
    Follower::new([path.as_ref().to_path_buf()], FollowOptions::default())
}
