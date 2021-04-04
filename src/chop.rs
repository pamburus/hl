// std imports
use std::collections::VecDeque;
use std::sync::Arc;

// third-party imports
use async_std::{
    io::{Read, ReadExt},
    stream::{Stream, StreamExt},
};
use async_stream::stream;
use futures_util::pin_mut;

// local imports
use crate::error::*;
use crate::pool::{Factory, Recycler, SQPool};

// ---

pub struct Chopper {
    delimiter: String,
    sbf: Arc<SegmentBufFactory>,
}

impl Chopper {
    /// Returns a new Chopper with the given parameters.
    pub fn new(sbf: Arc<SegmentBufFactory>, delimiter: String) -> Self {
        Self {
            delimiter: delimiter.clone(),
            sbf,
        }
    }

    /// Returns a stream of segments found in the input.
    pub fn chop<R>(self, mut input: R) -> impl Stream<Item = Result<Segment>>
    where
        R: Read + Unpin + 'static,
    {
        stream! {
            let mut current = self.sbf.new_buf();
            let mut partial = false;
            let mut done = false;
            while !done {
                let n = match input.read(&mut current.data[current.size..]).await {
                    Ok(value) => value,
                    Err(err) => {
                        yield Err(err.into());
                        break;
                    }
                };
                current.size += n;
                let full = current.size == current.data.capacity();
                let (next, st) = if n == 0 {
                    done = true;
                    (
                        SegmentBuf::zero(),
                        if partial {
                            Some(PartialPlacement::End)
                        } else {
                            None
                        },
                    )
                } else {
                    match self.split(&mut current) {
                         Some(next) => {
                            let result = (
                                next,
                                if partial {
                                    Some(PartialPlacement::End)
                                } else {
                                    None
                                },
                            );
                            partial = false;
                            result
                        }
                        None => {
                            if !full {
                                continue;
                            }
                            let st = if partial {
                                Some(PartialPlacement::Middle)
                            } else {
                                Some(PartialPlacement::Begin)
                            };
                            partial = true;
                            (self.sbf.new_buf(), st)
                        }
                    }
                };
                yield Ok(Segment::new(current.replace(next), st));
            }
        }
    }

    pub fn chop_jumbo<R>(
        self,
        input: R,
        max_segment_size: usize,
    ) -> impl Stream<Item = Result<Segment>>
    where
        R: Read + Unpin + 'static,
    {
        stream! {
            let mut fetched = VecDeque::<(SegmentBuf, PartialPlacement)>::new();
            let mut next = None;
            let mut done = false;
            let sbf = self.sbf.clone();
            let inner = self.chop(input);
            pin_mut!(inner);

            let complete = |n: Option<Result<Segment>>, next: &mut Option<Result<Segment>>, fetched: &mut VecDeque<(SegmentBuf, PartialPlacement)>| {
                if fetched.len() == 0 {
                    return n;
                }
                *next = n;
                let buf = fetched
                    .iter()
                    .flat_map(|(buf, _)| buf.data())
                    .cloned()
                    .collect::<Vec<u8>>();
                for (buf, _) in fetched.drain(..) {
                    sbf.recycle(buf);
                }
                return Some(Ok(Segment::Regular(buf.into())));
            };

            while !done {
                if let Some((buf, placement)) = fetched.pop_front() {
                    yield Ok(Segment::Partial(buf, placement));
                }
                if let Some(next) = next.take() {
                    yield next;
                }

                let mut total = 0;
                while let item = inner.next().await {
                    match item {
                        Some(Ok(Segment::Partial(buf, placement))) => {
                            total += buf.data().len();
                            fetched.push_back((buf, placement));
                            if placement == PartialPlacement::End {
                                match complete(None, &mut next, &mut fetched) {
                                    None => {
                                        done = true;
                                    }
                                    Some(item) => {
                                        yield item;
                                    }
                                }
                            }
                        }
                        n @ _ => {
                            match complete(n, &mut next, &mut fetched) {
                                None => {
                                    done = true;
                                }
                                Some(item) => {
                                    yield item;
                                }
                            }
                        }
                    }
                    if total > max_segment_size {
                        break;
                    }
                }
            }
        }
    }

    fn split(&self, current: &mut SegmentBuf) -> Option<SegmentBuf> {
        let k = self.delimiter.len();
        if current.size < k || k == 0 {
            return None;
        }

        for i in (0..current.size - k + 1).rev() {
            if current.data[i..].starts_with(self.delimiter.as_bytes()) {
                let n = current.size - i - k;
                let mut result = self.sbf.new_buf();
                if result.data.len() < n {
                    result.data.resize(n, 0);
                }
                if n > 0 {
                    result.data[..n].copy_from_slice(&current.data[i + k..i + k + n]);
                    result.size = n;
                    current.size -= n;
                }
                return Some(result);
            }
        }
        None
    }
}

// ---

/// Segment is an output of Scanner.
/// Complete segment cantains a whole number of tokens.
/// Incomplete segment contains a part of a token.
#[derive(Debug, Eq, PartialEq)]
pub enum Segment {
    Regular(SegmentBuf),
    Partial(SegmentBuf, PartialPlacement),
}

impl Segment {
    /// Returns a new Segment containing the given SegmentBuf.
    fn new(buf: SegmentBuf, placement: Option<PartialPlacement>) -> Self {
        if let Some(placement) = placement {
            Self::Partial(buf, placement)
        } else {
            Self::Regular(buf)
        }
    }
}

// ---

/// Defines partial segment placement in a sequence.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum PartialPlacement {
    Begin,
    Middle,
    End,
}

// ---

/// Contains a pre-allocated data buffer for a Segment and data size.
#[derive(Eq)]
pub struct SegmentBuf {
    data: Vec<u8>,
    size: usize,
}

impl SegmentBuf {
    /// Returns a reference to the contained data.
    pub fn data(&self) -> &[u8] {
        &self.data[..self.size]
    }

    /// Converts the SegmentBuf to a Vec<u8>.
    pub fn to_vec(mut self) -> Vec<u8> {
        self.data.resize(self.size, 0);
        self.data
    }

    fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        data.resize(capacity, 0);
        Self { data, size: 0 }
    }

    fn zero() -> Self {
        Self {
            data: Vec::new(),
            size: 0,
        }
    }

    fn reset(&mut self) {
        self.data.resize(self.data.capacity(), 0);
        self.size = 0;
    }

    fn resetted(mut self) -> Self {
        self.reset();
        self
    }

    fn replace(&mut self, mut other: Self) -> Self {
        std::mem::swap(self, &mut other);
        other
    }
}

impl PartialEq for SegmentBuf {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.data().eq(other.data())
    }
}

impl std::fmt::Debug for SegmentBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(s) = std::str::from_utf8(self.data()) {
            write!(f, "{:?}", s)
        } else {
            write!(f, "{:?}", self.data())
        }
    }
}

impl<T: AsRef<[u8]>> From<T> for SegmentBuf {
    fn from(data: T) -> Self {
        let size = data.as_ref().len();
        Self {
            data: data.as_ref().into(),
            size,
        }
    }
}

// ---

/// Constructs new SegmentBuf's with the configures size and recycles unneeded SegmentBuf's.
pub struct SegmentBufFactory {
    pool: SQPool<SegmentBuf, SBFFactory, SBFRecycler>,
}

impl SegmentBufFactory {
    /// Returns a new SegmentBufFactory with the given parameters.
    pub fn new(buf_size: usize) -> SegmentBufFactory {
        return SegmentBufFactory {
            pool: SQPool::new_with_factory(SBFFactory { buf_size }).with_recycler(SBFRecycler),
        };
    }

    /// Returns a new or recycled SegmentBuf.
    pub fn new_buf(&self) -> SegmentBuf {
        self.pool.checkout()
    }

    /// Recycles the given SegmentBuf.
    pub fn recycle(&self, buf: SegmentBuf) {
        self.pool.checkin(buf)
    }
}

// --

struct SBFFactory {
    buf_size: usize,
}

impl Factory<SegmentBuf> for SBFFactory {
    fn new(&self) -> SegmentBuf {
        SegmentBuf::new(self.buf_size)
    }
}
// --

struct SBFRecycler;

impl Recycler<SegmentBuf> for SBFRecycler {
    fn recycle(&self, buf: SegmentBuf) -> SegmentBuf {
        buf.resetted()
    }
}

// ---

/// Constructs new raw Vec<u8> buffers with the configured size.
pub struct BufFactory {
    pool: SQPool<Vec<u8>, RawBufFactory, RawBufRecycler>,
}

impl BufFactory {
    /// Returns a new BufFactory with the given parameters.
    pub fn new(buf_size: usize) -> Self {
        return Self {
            pool: SQPool::new()
                .with_factory(RawBufFactory { buf_size })
                .with_recycler(RawBufRecycler),
        };
    }

    /// Returns a new or recycled buffer.
    pub fn new_buf(&self) -> Vec<u8> {
        self.pool.checkout()
    }

    /// Recycles the given buffer.
    pub fn recycle(&self, buf: Vec<u8>) {
        self.pool.checkin(buf);
    }
}

// ---

struct RawBufFactory {
    buf_size: usize,
}

impl Factory<Vec<u8>> for RawBufFactory {
    fn new(&self) -> Vec<u8> {
        Vec::with_capacity(self.buf_size)
    }
}

// ---

struct RawBufRecycler;

impl Recycler<Vec<u8>> for RawBufRecycler {
    fn recycle(&self, mut buf: Vec<u8>) -> Vec<u8> {
        buf.resize(0, 0);
        buf
    }
}
