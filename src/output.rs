// std imports
use std::io::Write;
use std::sync::Arc;

// third-party imports
use clap::ValueEnum;
use serde::Deserialize;

// local imports
use crate::{
    pool::{Pool, Pooled, Recycler},
    scanning::SegmentBuf,
};

mod closed;

pub use closed::on_stdout_closed;

pub type OutputStream = Box<dyn Write + Send + Sync>;

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputDelimiter {
    Newline,
    Nul,
}

// ---

/// How many times larger than the configured size an output buffer can be to be kept for reuse.
///
/// With colors, a full input buffer of compact records with short fields formats to about 4 times its size, and to
/// about 10 times with field expansion, so a buffer of the configured size grows up to 16 times by doubling.
/// A buffer larger than that was grown by an entry much larger than the buffer size.
const OUTPUT_BUF_KEEP_FACTOR: usize = 16;

/// Constructs new output buffers with the configured capacity and takes back the ones dropped, see [`BufRecycler`].
pub(crate) type BufFactory = Pool<BufRecycler>;

/// An output buffer, which returns to the [`BufFactory`] it came from, if any, when dropped.
pub(crate) type OutputBuf = Pooled<BufRecycler>;

impl BufFactory {
    /// Returns a new BufFactory with the given parameters.
    #[inline]
    pub fn new(buf_size: usize) -> Self {
        Self::with_recycler(BufRecycler { buf_size })
    }

    /// Returns a new or recycled empty buffer, which returns to the factory when dropped.
    #[inline]
    pub fn new_buf(self: &Arc<Self>) -> OutputBuf {
        self.take()
    }
}

impl From<Vec<u8>> for OutputBuf {
    /// Returns an output buffer that does not belong to any pool.
    #[inline]
    fn from(buf: Vec<u8>) -> Self {
        Self::detached(buf)
    }
}

/// Data to be written, in either an output buffer or the buffer it was read into, so both can be passed on
/// together without copying. The buffer returns to the pool it came from, if any, when dropped.
pub(crate) enum Chunk {
    /// Data in an output buffer, such as formatted entries.
    Output(OutputBuf),
    /// Data in the buffer it was read into, such as a part of a log entry too large to be read at once, shown as is.
    Segment(SegmentBuf),
}

impl Chunk {
    /// Returns the data.
    #[inline]
    pub fn data(&self) -> &[u8] {
        match self {
            Self::Output(buf) => buf,
            Self::Segment(segment) => segment.data(),
        }
    }
}

/// Creates the buffers of a [`BufFactory`] and decides which returned ones it keeps.
///
/// Only buffers from the configured size up to [`OUTPUT_BUF_KEEP_FACTOR`] times larger are kept, so a buffer grown by
/// a large entry does not hold its memory for later entries, and no buffer smaller than the configured size is reused.
pub(crate) struct BufRecycler {
    buf_size: usize,
}

impl Recycler for BufRecycler {
    type Item = Vec<u8>;

    #[inline]
    fn create(&self) -> Vec<u8> {
        Vec::with_capacity(self.buf_size)
    }

    #[inline]
    fn reuse(&self, buf: &mut Vec<u8>) {
        buf.clear();
    }

    #[inline]
    fn keep(&self, buf: &Vec<u8>) -> bool {
        (self.buf_size..=self.buf_size.saturating_mul(OUTPUT_BUF_KEEP_FACTOR)).contains(&buf.capacity())
    }
}

// ---

#[cfg(test)]
mod tests;
