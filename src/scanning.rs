// std imports
use std::convert::From;
use std::io::Read;
use std::sync::Arc;

// third-party imports
use crossbeam_queue::SegQueue;

// local imports
use crate::error::*;

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

/// Segment is an output of Scanner.
/// Complete segment cantains a whole number of tokens.
/// Incomplete segment contains a part of a token.
#[derive(Debug, Eq, PartialEq)]
pub enum Segment {
    Complete(SegmentBuf),
    Incomplete(SegmentBuf),
}

impl Segment {
    /// Returns a new Segment containing the given SegmentBuf.
    fn new(buf: SegmentBuf, partial: bool) -> Self {
        if partial {
            Self::Incomplete(buf)
        } else {
            Self::Complete(buf)
        }
    }
}

// ---

/// Constructs new SegmentBuf's with the configures size and recycles unneeded SegmentBuf's.
pub struct SegmentBufFactory {
    buf_size: usize,
    recycled: SegQueue<SegmentBuf>,
}

impl SegmentBufFactory {
    /// Returns a new SegmentBufFactory with the given parameters.
    pub fn new(buf_size: usize) -> Self {
        return Self {
            buf_size,
            recycled: SegQueue::new(),
        };
    }

    /// Returns a new or recycled SegmentBuf.
    pub fn new_segment(&self) -> SegmentBuf {
        match self.recycled.pop() {
            Some(buf) => buf.resetted(),
            None => SegmentBuf::new(self.buf_size),
        }
    }

    /// Recycles the given SegmentBuf.
    pub fn recycle(&self, buf: SegmentBuf) {
        self.recycled.push(buf);
    }
}

// ---

/// Constructs new raw Vec<u8> buffers with the configured size.
pub struct BufFactory {
    buf_size: usize,
    recycled: SegQueue<Vec<u8>>,
}

impl BufFactory {
    /// Returns a new BufFactory with the given parameters.
    pub fn new(buf_size: usize) -> Self {
        return Self {
            buf_size,
            recycled: SegQueue::new(),
        };
    }

    /// Returns a new or recycled buffer.
    pub fn new_buf(&self) -> Vec<u8> {
        match self.recycled.pop() {
            Some(mut buf) => {
                buf.resize(0, 0);
                buf
            }
            None => Vec::with_capacity(self.buf_size),
        }
    }

    /// Recycles the given buffer.
    pub fn recycle(&self, buf: Vec<u8>) {
        self.recycled.push(buf);
    }
}

// ---

/// Iterates over the input stream and returns segments containing one or more tokens.
pub struct ScannerIter<'a, 'b> {
    scanner: &'a Scanner,
    input: &'b mut dyn Read,
    next: SegmentBuf,
    partial: bool,
    done: bool,
}

impl<'a, 'b> ScannerIter<'a, 'b> {
    fn new(scanner: &'a Scanner, input: &'b mut dyn Read) -> Self {
        return Self {
            scanner,
            input,
            next: scanner.sf.new_segment(),
            partial: false,
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

            let (next, partial) = if n == 0 {
                self.done = true;
                (SegmentBuf::zero(), self.partial)
            } else {
                match self.split() {
                    Some(next) => {
                        let result = (next, self.partial);
                        self.partial = false;
                        result
                    }
                    None => {
                        if !full {
                            continue;
                        }
                        self.partial = true;
                        (self.scanner.sf.new_segment(), true)
                    }
                }
            };

            let result = self.next.replace(next);
            return if result.size != 0 {
                Some(Ok(Segment::new(result, partial)))
            } else {
                None
            };
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
}
