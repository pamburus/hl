use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::Error;
use crate::identity::FileId;
use crate::options::FollowOptions;

// ---

/// The input type of a followed path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Regular,
    Pipe,
}

// ---

#[derive(Debug)]
enum State {
    /// fd open, reading from `offset`.
    Following,
    /// Identity changed; draining old fd to EOF before switching.
    Draining,
    /// Path missing or never existed; retrying.
    Waiting,
}

// ---

/// Per-source state machine that owns the fd, offset, and identity tracking.
/// This is the heart of the no-data-loss guarantee: every wake reconciles
/// from authoritative `stat` before reading (research §2, §3).
pub struct FollowedSource {
    pub path: PathBuf,
    pub kind: SourceKind,
    fd: Option<File>,
    offset: u64,
    last_size: u64,
    identity: Option<FileId>,
    state: State,
    options: FollowOptions,
}

impl FollowedSource {
    /// Open a new `FollowedSource` for a regular file or FIFO.
    pub fn open(path: PathBuf, options: FollowOptions) -> Result<Self, Error> {
        use std::fs;

        let meta = fs::metadata(&path);
        let kind = match meta {
            Ok(ref m) => {
                let ft = m.file_type();
                if ft.is_file() {
                    SourceKind::Regular
                } else if ft.is_fifo_or_pipe() {
                    SourceKind::Pipe
                } else {
                    return Err(Error::UnsupportedInput {
                        path: path.clone(),
                        kind: file_type_name(ft),
                    });
                }
            }
            Err(_) => {
                // Path doesn't exist yet — start in Waiting state.
                return Ok(Self {
                    path,
                    kind: SourceKind::Regular,
                    fd: None,
                    offset: 0,
                    last_size: 0,
                    identity: None,
                    state: State::Waiting,
                    options,
                });
            }
        };

        match kind {
            SourceKind::Regular => {
                let meta = meta.unwrap();
                let last_size = meta.len();
                let mut fd = File::open(&path).map_err(Error::Io)?;
                // Seek to current EOF so we only deliver new appends (FR-008
                // handoff).
                fd.seek(SeekFrom::Start(last_size)).map_err(Error::Io)?;
                let identity = FileId::from_file(&fd).ok();
                Ok(Self {
                    path,
                    kind,
                    fd: Some(fd),
                    offset: last_size,
                    last_size,
                    identity,
                    state: State::Following,
                    options,
                })
            }
            SourceKind::Pipe => {
                let fd = File::open(&path).map_err(Error::Io)?;
                Ok(Self {
                    path,
                    kind,
                    fd: Some(fd),
                    offset: 0,
                    last_size: 0,
                    identity: None,
                    state: State::Following,
                    options,
                })
            }
        }
    }

    /// Returns true if this source has permanently ended (not retrying).
    pub fn is_done(&self) -> bool {
        matches!(self.state, State::Waiting) && !self.options.retry_missing
    }

    /// Reconcile from filesystem state and read any available bytes into `buf`.
    ///
    /// Returns `Ok(0)` if no bytes are available yet (not EOF — the caller
    /// should wait for the next wake signal). Returns `Ok(n)` with data, or
    /// `Err` on I/O error.
    pub fn read_available(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        match self.kind {
            SourceKind::Regular => self.read_regular(buf),
            SourceKind::Pipe => self.read_pipe(buf),
        }
    }

    fn read_regular(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        loop {
            match &self.state {
                State::Waiting => {
                    // Try to open the path.
                    match File::open(&self.path) {
                        Ok(fd) => {
                            let identity = FileId::from_file(&fd).ok();
                            let size = fd.metadata().map(|m| m.len()).unwrap_or(0);
                            self.fd = Some(fd);
                            self.offset = 0;
                            self.last_size = size;
                            self.identity = identity;
                            self.state = State::Following;
                            // Fall through to Following.
                        }
                        Err(_) => {
                            // Still missing.
                            return Ok(0);
                        }
                    }
                }
                State::Draining => {
                    // Drain the old fd to EOF.
                    if let Some(fd) = &mut self.fd {
                        match fd.read(buf) {
                            Ok(0) => {
                                // Old fd fully drained — open new file at offset 0.
                                self.fd = None;
                                match File::open(&self.path) {
                                    Ok(new_fd) => {
                                        let identity = FileId::from_file(&new_fd).ok();
                                        let size = new_fd.metadata().map(|m| m.len()).unwrap_or(0);
                                        self.fd = Some(new_fd);
                                        self.offset = 0;
                                        self.last_size = size;
                                        self.identity = identity;
                                        self.state = State::Following;
                                        // Fall through to Following.
                                    }
                                    Err(_) => {
                                        self.state = State::Waiting;
                                        return Ok(0);
                                    }
                                }
                            }
                            Ok(n) => {
                                self.offset += n as u64;
                                return Ok(n);
                            }
                            Err(e) => return Err(Error::Io(e)),
                        }
                    } else {
                        // No old fd — transition to Following or Waiting.
                        match File::open(&self.path) {
                            Ok(new_fd) => {
                                let identity = FileId::from_file(&new_fd).ok();
                                let size = new_fd.metadata().map(|m| m.len()).unwrap_or(0);
                                self.fd = Some(new_fd);
                                self.offset = 0;
                                self.last_size = size;
                                self.identity = identity;
                                self.state = State::Following;
                            }
                            Err(_) => {
                                self.state = State::Waiting;
                                return Ok(0);
                            }
                        }
                    }
                }
                State::Following => {
                    // Re-derive ground truth from stat.
                    let path_meta = std::fs::metadata(&self.path);

                    match path_meta {
                        Err(_) => {
                            // Path disappeared.
                            self.state = State::Waiting;
                            self.fd = None;
                            return Ok(0);
                        }
                        Ok(meta) => {
                            let current_size = meta.len();
                            let current_id = FileId::from_path(&self.path).ok();

                            // Check for identity change (rotation / replacement).
                            let identity_changed = match (self.identity, current_id) {
                                (Some(old), Some(new)) => old != new,
                                _ => false,
                            };

                            if identity_changed {
                                log::debug!("fsmon: {} identity changed, draining old fd", self.path.display());
                                // Stay in Following state momentarily but flip to
                                // Draining to drain the old fd first.
                                self.state = State::Draining;
                                // Loop back to handle Draining.
                                continue;
                            }

                            // Same identity: check for truncation.
                            if current_size < self.offset {
                                log::debug!("fsmon: {} truncated, seeking to 0", self.path.display());
                                if let Some(fd) = &mut self.fd {
                                    fd.seek(SeekFrom::Start(0)).map_err(Error::Io)?;
                                }
                                self.offset = 0;
                                self.last_size = current_size;
                                // Fall through to read.
                            }

                            // Read appended bytes.
                            if let Some(fd) = &mut self.fd {
                                match fd.read(buf) {
                                    Ok(0) => {
                                        // At EOF — no data yet.
                                        self.last_size = current_size;
                                        return Ok(0);
                                    }
                                    Ok(n) => {
                                        self.offset += n as u64;
                                        self.last_size = current_size;
                                        return Ok(n);
                                    }
                                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                                        continue;
                                    }
                                    Err(e) => return Err(Error::Io(e)),
                                }
                            } else {
                                return Ok(0);
                            }
                        }
                    }
                }
            }
        }
    }

    fn read_pipe(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if let Some(fd) = &mut self.fd {
            match fd.read(buf) {
                Ok(0) => Ok(0), // EOF on pipe — writer closed
                Ok(n) => Ok(n),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => Ok(0),
                Err(e) => Err(Error::Io(e)),
            }
        } else {
            Ok(0)
        }
    }
}

// ---

fn file_type_name(ft: std::fs::FileType) -> &'static str {
    if ft.is_dir() {
        "directory"
    } else if ft.is_symlink() {
        "symlink"
    } else {
        "special file"
    }
}

// Platform-specific FIFO/pipe detection.
trait FileTypeExt {
    fn is_fifo_or_pipe(&self) -> bool;
}

impl FileTypeExt for std::fs::FileType {
    #[cfg(unix)]
    fn is_fifo_or_pipe(&self) -> bool {
        use std::os::unix::fs::FileTypeExt;
        self.is_fifo()
    }

    #[cfg(not(unix))]
    fn is_fifo_or_pipe(&self) -> bool {
        false
    }
}
