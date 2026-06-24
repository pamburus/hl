//! Robust file following (`tail -F` parity) for hl.
//!
//! Two layers:
//! - **Facade** (`follow`/`Follower`): simple per-source byte stream hiding
//!   rotation, truncation, and deletion. Most consumers use this.
//! - **Core** (`watch::Watcher`): advisory event stream with per-path engine
//!   routing. For advanced consumers (e.g., tail-style status messages).
//!
//! # Quick start
//!
//! ```rust,no_run
//! use fsmon::follow;
//! use std::io::Read;
//!
//! let mut reader = follow("/var/log/app.log").unwrap().into_reader();
//! let mut buf = vec![0u8; 64 * 1024];
//! loop {
//!     let n = reader.read(&mut buf).unwrap();
//!     if n == 0 { break; }
//!     // process buf[..n]
//! }
//! ```

use std::path::PathBuf;

// ---

pub mod classify;
pub mod follow;
pub mod options;
pub mod watch;

mod identity;

// ---

pub use follow::follow;
pub use follow::{Chunk, Follower};
pub use options::{FallbackPolicy, FollowOptions};
pub use watch::Engine;

// ---

pub type Result<T> = std::result::Result<T, Error>;

// ---

/// Stable handle identifying one followed path in a multi-source `Follower`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceId(pub(crate) usize);

// ---

/// Errors from the `fsmon` crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Unsupported input type (directory / socket / device file). Only regular
    /// files and FIFOs are accepted (FR-026).
    #[error("unsupported input type '{kind}' for path: {}", path.display())]
    UnsupportedInput { path: PathBuf, kind: &'static str },

    /// Underlying watch backend failure.
    #[error("watch backend error: {0}")]
    Watch(#[from] notify::Error),

    /// I/O error while opening or reading a followed file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
