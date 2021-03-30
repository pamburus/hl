// std imports
use std::collections::VecDeque;
use std::convert::From;
use std::io::Read;
use std::sync::Arc;

// local imports
use crate::error::*;
use crate::pool::{Factory, Recycler, SQPool};

// ---

/// Scans input stream and splits it into segments containing a whole number of tokens delimited by the given delimiter.
/// If a single token exceeds size of a buffer allocated by SegmentBufFactory, it is split into multiple Incomplete segments.
pub struct Scanner {
    delimiter: String,
    sf: Arc<SegmentBufFactory>,
}

impl Scanner {
    /// Returns a new Scanner with the given parameters.
    pub fn new(sf: Arc<SegmentBufFactory>, delimiter: String) -> Self {
        Self {
            delimiter: delimiter.clone(),
            sf,
        }
    }

    /// Returns an iterator over segments found in the input.
    pub fn items<'a, 'b>(&'a self, input: &'b mut dyn Read) -> ScannerIter<'a, 'b> {
        return ScannerIter::new(self, input);
    }
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
    #[inline(always)]
    pub fn data(&self) -> &[u8] {
        &self.data[..self.size]
    }

    /// Converts the SegmentBuf to a Vec<u8>.
    #[inline(always)]
    pub fn to_vec(mut self) -> Vec<u8> {
        self.data.resize(self.size, 0);
        self.data
    }

    #[inline]
    fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        data.resize(capacity, 0);
        Self { data, size: 0 }
    }

    #[inline(always)]
    fn zero() -> Self {
        Self {
            data: Vec::new(),
            size: 0,
        }
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.data.resize(self.data.capacity(), 0);
        self.size = 0;
    }

    #[inline(always)]
    fn resetted(mut self) -> Self {
        self.reset();
        self
    }

    #[inline(always)]
    fn replace(&mut self, mut other: Self) -> Self {
        std::mem::swap(self, &mut other);
        other
    }
}

impl PartialEq for SegmentBuf {
    #[inline(always)]
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
    #[inline(always)]
    fn from(data: T) -> Self {
        let size = data.as_ref().len();
        Self {
            data: data.as_ref().into(),
            size,
        }
    }
}

// ---

/// Segment is an output of Scanner.
/// Complete segment cantains a whole number of tokens.
/// Incomplete segment contains a part of a token.
#[derive(Debug, Eq, PartialEq)]
pub enum Segment {
    Complete(SegmentBuf),
    Incomplete(SegmentBuf, PartialPlacement),
}

impl Segment {
    /// Returns a new Segment containing the given SegmentBuf.
    #[inline(always)]
    fn new(buf: SegmentBuf, placement: Option<PartialPlacement>) -> Self {
        if let Some(placement) = placement {
            Self::Incomplete(buf, placement)
        } else {
            Self::Complete(buf)
        }
    }
}

// ---

/// Defines partial segment placement in a sequence.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum PartialPlacement {
    First,
    Next,
    Last,
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
    #[inline(always)]
    pub fn new_segment(&self) -> SegmentBuf {
        self.pool.checkout()
    }

    /// Recycles the given SegmentBuf.
    #[inline(always)]
    pub fn recycle(&self, buf: SegmentBuf) {
        self.pool.checkin(buf)
    }
}

// --

struct SBFFactory {
    buf_size: usize,
}

impl Factory<SegmentBuf> for SBFFactory {
    #[inline(always)]
    fn new(&self) -> SegmentBuf {
        SegmentBuf::new(self.buf_size)
    }
}
// --

struct SBFRecycler;

impl Recycler<SegmentBuf> for SBFRecycler {
    #[inline(always)]
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
    #[inline(always)]
    pub fn new_buf(&self) -> Vec<u8> {
        self.pool.checkout()
    }

    /// Recycles the given buffer.
    #[inline(always)]
    pub fn recycle(&self, buf: Vec<u8>) {
        self.pool.checkin(buf);
    }
}

// ---

struct RawBufFactory {
    buf_size: usize,
}

impl Factory<Vec<u8>> for RawBufFactory {
    #[inline(always)]
    fn new(&self) -> Vec<u8> {
        Vec::with_capacity(self.buf_size)
    }
}

// ---

struct RawBufRecycler;

impl Recycler<Vec<u8>> for RawBufRecycler {
    #[inline(always)]
    fn recycle(&self, mut buf: Vec<u8>) -> Vec<u8> {
        buf.resize(0, 0);
        buf
    }
}

// ---

/// Iterates over the input stream and returns segments containing one or more tokens.
pub struct ScannerIter<'a, 'b> {
    scanner: &'a Scanner,
    input: &'b mut dyn Read,
    next: SegmentBuf,
    placement: Option<PartialPlacement>,
    done: bool,
}

impl<'a, 'b> ScannerIter<'a, 'b> {
    pub fn with_max_segment_size(self, max_segment_size: usize) -> ScannerJumboIter<'a, 'b> {
        ScannerJumboIter::new(self, max_segment_size)
    }

    fn new(scanner: &'a Scanner, input: &'b mut dyn Read) -> Self {
        return Self {
            scanner,
            input,
            next: scanner.sf.new_segment(),
            placement: None,
            done: false,
        };
    }

    fn split(&mut self) -> Option<SegmentBuf> {
        let k = self.scanner.delimiter.len();
        if self.next.size < k || k == 0 {
            return None;
        }

        for i in (0..self.next.size - k + 1).rev() {
            if self.next.data[i..].starts_with(self.scanner.delimiter.as_bytes()) {
                let n = self.next.size - i - k;
                let mut result = self.scanner.sf.new_segment();
                if result.data.len() < n {
                    result.data.resize(n, 0);
                }
                if n > 0 {
                    result.data[..n].copy_from_slice(&self.next.data[i + k..i + k + n]);
                    result.size = n;
                    self.next.size -= n;
                }
                return Some(result);
            }
        }
        None
    }
}

impl<'a, 'b> Iterator for ScannerIter<'a, 'b> {
    type Item = Result<Segment>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            let n = match self.input.read(&mut self.next.data[self.next.size..]) {
                Ok(value) => value,
                Err(err) => {
                    self.done = true;
                    return Some(Err(err.into()));
                }
            };
            self.next.size += n;
            let full = self.next.size == self.next.data.capacity();

            let (next, placement) = if n == 0 {
                self.done = true;
                (
                    SegmentBuf::zero(),
                    self.placement.and(Some(PartialPlacement::Last)),
                )
            } else {
                match self.split() {
                    Some(next) => {
                        let result = (next, self.placement.and(Some(PartialPlacement::Last)));
                        self.placement = None;
                        result
                    }
                    None => {
                        if !full {
                            continue;
                        }
                        self.placement = self
                            .placement
                            .and(Some(PartialPlacement::Next))
                            .or(Some(PartialPlacement::First));
                        (self.scanner.sf.new_segment(), self.placement)
                    }
                }
            };

            let result = self.next.replace(next);
            self.placement = placement;
            return if result.size != 0 {
                Some(Ok(Segment::new(result, placement)))
            } else {
                None
            };
        }
    }
}

// ---

/// Iterates over the input stream and returns segments containing tokens.
/// Unlike ScannerIter ScannerJumboIter joins incomplete segments into a single complete segment
/// if its size does not exceed max_segment_size.
pub struct ScannerJumboIter<'a, 'b> {
    inner: ScannerIter<'a, 'b>,
    max_segment_size: usize,
    fetched: VecDeque<(SegmentBuf, PartialPlacement)>,
    next: Option<Result<Segment>>,
}

impl<'a, 'b> ScannerJumboIter<'a, 'b> {
    fn new(inner: ScannerIter<'a, 'b>, max_segment_size: usize) -> Self {
        return Self {
            inner,
            max_segment_size,
            fetched: VecDeque::new(),
            next: None,
        };
    }

    fn complete(&mut self, next: Option<Result<Segment>>) -> Option<Result<Segment>> {
        if self.fetched.len() == 0 {
            return next;
        }

        self.next = next;
        let buf = self
            .fetched
            .iter()
            .flat_map(|(buf, _)| buf.data())
            .cloned()
            .collect::<Vec<u8>>();
        for (buf, _) in self.fetched.drain(..) {
            self.inner.scanner.sf.recycle(buf);
        }

        return Some(Ok(Segment::Complete(buf.into())));
    }
}

impl<'a, 'b> Iterator for ScannerJumboIter<'a, 'b> {
    type Item = Result<Segment>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((buf, placement)) = self.fetched.pop_front() {
                return Some(Ok(Segment::Incomplete(buf, placement)));
            }
            if let Some(next) = self.next.take() {
                return Some(next);
            }

            let mut total = 0;
            loop {
                let next = self.inner.next();
                match next {
                    Some(Ok(Segment::Incomplete(buf, placement))) => {
                        total += buf.data().len();
                        self.fetched.push_back((buf, placement));
                        if placement == PartialPlacement::Last {
                            return self.complete(None);
                        }
                    }
                    next @ _ => {
                        return self.complete(next);
                    }
                };
                if total > self.max_segment_size {
                    break;
                }
            }
        }
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_token() {
        let sf = Arc::new(SegmentBufFactory::new(20));
        let scanner = Scanner::new(sf.clone(), "/".into());
        let mut data = std::io::Cursor::new(b"token");
        let tokens = scanner
            .items(&mut data)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(tokens, vec![Segment::Complete(b"token".into())])
    }

    #[test]
    fn test_empty_token_and_small_token() {
        let sf = Arc::new(SegmentBufFactory::new(20));
        let scanner = Scanner::new(sf.clone(), "/".into());
        let mut data = std::io::Cursor::new(b"/token");
        let tokens = scanner
            .items(&mut data)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            tokens,
            vec![
                Segment::Complete(b"/".into()),
                Segment::Complete(b"token".into())
            ]
        )
    }

    #[test]
    fn test_small_token_and_empty_token() {
        let sf = Arc::new(SegmentBufFactory::new(20));
        let scanner = Scanner::new(sf.clone(), "/".into());
        let mut data = std::io::Cursor::new(b"token/");
        let tokens = scanner
            .items(&mut data)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(tokens, vec![Segment::Complete(b"token/".into())])
    }

    #[test]
    fn test_two_small_tokens() {
        let sf = Arc::new(SegmentBufFactory::new(20));
        let scanner = Scanner::new(sf.clone(), "/".into());
        let mut data = std::io::Cursor::new(b"test/token/");
        let tokens = scanner
            .items(&mut data)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(tokens, vec![Segment::Complete(b"test/token/".into())])
    }

    #[test]
    fn test_two_tokens_over_segment_size() {
        let sf = Arc::new(SegmentBufFactory::new(10));
        let scanner = Scanner::new(sf.clone(), "/".into());
        let mut data = std::io::Cursor::new(b"test/token/");
        let tokens = scanner
            .items(&mut data)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            tokens,
            vec![
                Segment::Complete(b"test/".into()),
                Segment::Complete(b"token/".into())
            ]
        )
    }

    #[test]
    fn test_jumbo_1() {
        let sf = Arc::new(SegmentBufFactory::new(2));
        let scanner = Scanner::new(sf.clone(), "/".into());
        let mut data = std::io::Cursor::new(b"test/token/very/large/");
        let tokens = scanner
            .items(&mut data)
            .with_max_segment_size(6)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            tokens,
            vec![
                Segment::Complete(b"test/".into()),
                Segment::Complete(b"token/".into()),
                Segment::Complete(b"very/".into()),
                Segment::Complete(b"large/".into()),
            ]
        )
    }
}
