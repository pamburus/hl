// std imports
use std::cmp::min;
use std::collections::VecDeque;
use std::convert::From;
use std::io::Read;
use std::ops::{Deref, Range};
use std::sync::Arc;

// third-party imports
use crossbeam_queue::SegQueue;
use memchr::{memchr, memrchr};
use serde::{Deserialize, Serialize};

// local imports
use crate::error::*;

// ---

mod json;

// Re-export JSON delimiter
pub use json::JsonDelimiter;

/// Scans input stream and splits it into segments containing a whole number of tokens delimited by the given delimiter.
/// If a single token exceeds size of a buffer allocated by SegmentBufFactory, it is split into multiple Incomplete segments.
pub struct Scanner<D> {
    delimiter: D,
    sf: Arc<SegmentBufFactory>,
}

impl<D: Delimit> Scanner<D> {
    /// Returns a new Scanner with the given parameters.
    #[inline]
    pub fn new(sf: Arc<SegmentBufFactory>, delimiter: D) -> Self {
        Self { delimiter, sf }
    }

    /// Returns an iterator over segments found in the input.
    #[inline]
    pub fn items<'a, 'b>(&'a self, input: &'b mut dyn Read) -> ScannerIter<'a, 'b, D> {
        ScannerIter::new(self, input)
    }
}

// ---

/// Defines a token delimiter for Scanner.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Delimiter {
    Byte(u8),
    Bytes(Arc<[u8]>),
    Char(char),
    Str(Arc<str>),
    SmartNewLine,
    Json,
}

impl Default for Delimiter {
    #[inline]
    fn default() -> Self {
        Self::SmartNewLine
    }
}

impl From<u8> for Delimiter {
    #[inline]
    fn from(d: u8) -> Self {
        Self::Byte(d)
    }
}

impl From<Arc<[u8]>> for Delimiter {
    #[inline]
    fn from(d: Arc<[u8]>) -> Self {
        Self::Bytes(d)
    }
}

impl From<Vec<u8>> for Delimiter {
    #[inline]
    fn from(d: Vec<u8>) -> Self {
        Self::Bytes(d.into())
    }
}

impl From<&[u8]> for Delimiter {
    #[inline]
    fn from(d: &[u8]) -> Self {
        Self::Bytes(d.into())
    }
}

impl From<char> for Delimiter {
    #[inline]
    fn from(d: char) -> Self {
        Self::Char(d)
    }
}

impl From<&str> for Delimiter {
    #[inline]
    fn from(d: &str) -> Self {
        Self::Str(d.into())
    }
}

impl From<Arc<str>> for Delimiter {
    #[inline]
    fn from(d: Arc<str>) -> Self {
        Self::Str(d)
    }
}

impl From<String> for Delimiter {
    #[inline]
    fn from(d: String) -> Self {
        Self::Str(d.into())
    }
}

impl From<SmartNewLine> for Delimiter {
    #[inline]
    fn from(_: SmartNewLine) -> Self {
        Self::SmartNewLine
    }
}

impl Delimit for Delimiter {
    type Searcher = Arc<dyn Search>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        match self {
            Self::Byte(b) => Arc::new(b.into_searcher()),
            Self::Bytes(b) => Arc::new(b.into_searcher()),
            Self::Char(c) => Arc::new(c.into_searcher()),
            Self::Str(s) => Arc::new(s.into_searcher()),
            Self::SmartNewLine => Arc::new(SmartNewLine.into_searcher()),
            Self::Json => Arc::new(JsonDelimiter.into_searcher()),
        }
    }
}

// ---

/// Defines a trait for token delimiters for Scanner.
pub trait Delimit: Clone {
    type Searcher: Search;

    fn into_searcher(self) -> Self::Searcher;
}

impl Delimit for u8 {
    type Searcher = u8;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        self
    }
}

impl Delimit for &[u8] {
    type Searcher = SubStrSearcher<Self>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        SubStrSearcher::new(self)
    }
}

impl Delimit for char {
    type Searcher = SubStrSearcher<heapless::Vec<u8, 4>>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        let mut buf = [0; 4];
        self.encode_utf8(&mut buf);
        SubStrSearcher::new(heapless::Vec::from_slice(&buf[..self.len_utf8()]).unwrap())
    }
}

impl Delimit for &str {
    type Searcher = SubStrSearcher<Self>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        SubStrSearcher::new(self)
    }
}

impl Delimit for &String {
    type Searcher = SubStrSearcher<Self>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        SubStrSearcher::new(self)
    }
}

impl Delimit for String {
    type Searcher = SubStrSearcher<Self>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        SubStrSearcher::new(self)
    }
}

impl Delimit for Vec<u8> {
    type Searcher = SubStrSearcher<Self>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        SubStrSearcher::new(self)
    }
}

impl Delimit for Arc<[u8]> {
    type Searcher = SubStrSearcher<Self>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        SubStrSearcher::new(self)
    }
}

impl Delimit for Arc<str> {
    type Searcher = SubStrSearcher<Self>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        SubStrSearcher::new(self)
    }
}

impl Delimit for &Delimiter {
    type Searcher = Arc<dyn Search>;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        self.clone().into_searcher()
    }
}

// ---

/// Defines a smart new line delimiter that can be either LF or CRLF.
#[derive(Clone)]
pub struct SmartNewLine;

impl Delimit for SmartNewLine {
    type Searcher = SmartNewLineSearcher;

    #[inline(always)]
    fn into_searcher(self) -> Self::Searcher {
        Self::Searcher {}
    }
}

// ---

/// Defines a token delimiter search algorithm.
pub trait Search {
    /// Searches for the delimiter in the buffer from the right.
    #[must_use]
    fn search_r(&self, buf: &[u8], edge: bool) -> Option<Range<usize>>;

    /// Searches for the delimiter in the buffer from the left.
    #[must_use]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>>;

    /// Searches for a partial match of the delimiter at the right edge of the buffer.
    #[must_use]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize>;

    /// Searches for a partial match of the delimiter at the left edge of the buffer.
    #[must_use]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize>;
}

impl Search for u8 {
    #[inline(always)]
    fn search_r(&self, buf: &[u8], _: bool) -> Option<Range<usize>> {
        memrchr(*self, buf).map(|x| x..x + 1)
    }

    #[inline(always)]
    fn search_l(&self, buf: &[u8], _: bool) -> Option<Range<usize>> {
        memchr(*self, buf).map(|x| x..x + 1)
    }

    #[inline(always)]
    fn partial_match_l(&self, _: &[u8]) -> Option<usize> {
        None
    }

    #[inline(always)]
    fn partial_match_r(&self, _: &[u8]) -> Option<usize> {
        None
    }
}

impl Search for Box<dyn Search> {
    #[inline(always)]
    fn search_r(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        self.as_ref().search_r(buf, edge)
    }

    #[inline(always)]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        self.as_ref().search_l(buf, edge)
    }

    #[inline(always)]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        self.as_ref().partial_match_r(buf)
    }

    #[inline(always)]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        self.as_ref().partial_match_l(buf)
    }
}

impl Search for Arc<dyn Search> {
    #[inline(always)]
    fn search_r(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        self.as_ref().search_r(buf, edge)
    }

    #[inline(always)]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        self.as_ref().search_l(buf, edge)
    }

    #[inline(always)]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        self.as_ref().partial_match_r(buf)
    }

    #[inline(always)]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        self.as_ref().partial_match_l(buf)
    }
}

// ---

// Extends Search with a split method.
pub trait SearchExt: Search {
    #[inline]
    fn split<'a, 'b>(&'a self, buf: &'b [u8]) -> SplitIter<'a, 'b, Self>
    where
        Self: Sized,
    {
        SplitIter {
            searcher: self,
            buf,
            pos: 0,
        }
    }
}

impl<T: Search> SearchExt for T {}

// Iterates over the input buffer and returns slices of the buffer separated by the delimiter.
pub struct SplitIter<'a, 'b, S: Search + ?Sized> {
    searcher: &'a S,
    buf: &'b [u8],
    pos: usize,
}

impl<'a, 'b, S: Search> Iterator for SplitIter<'a, 'b, S> {
    type Item = &'b [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.buf.len() {
            return None;
        }

        let buf = &self.buf[self.pos..];
        let range = self.searcher.search_l(buf, true);
        if let Some(range) = range {
            self.pos += range.end;
            Some(&buf[..range.start])
        } else {
            self.pos = self.buf.len();
            Some(buf)
        }
    }
}

// ---

/// Searches for a substring in a byte slice.
pub struct SubStrSearcher<D> {
    delimiter: D,
}

impl<D> SubStrSearcher<D>
where
    D: Deref,
    D::Target: AsRef<[u8]>,
{
    #[inline]
    pub fn new(delimiter: D) -> Self {
        Self { delimiter }
    }

    #[inline]
    fn len(&self) -> usize {
        self.delimiter.deref().as_ref().len()
    }
}

impl<D> Search for SubStrSearcher<D>
where
    D: Deref,
    D::Target: AsRef<[u8]>,
{
    #[inline]
    fn search_r(&self, buf: &[u8], _edge: bool) -> Option<Range<usize>> {
        let needle = self.delimiter.deref().as_ref();
        if needle.is_empty() {
            return None;
        }

        let b = needle[0];
        let mut pos = buf.len();
        loop {
            if let Some(i) = memrchr(b, &buf[..pos]) {
                pos = i;
            } else {
                return None;
            }
            if buf[pos..].starts_with(needle) {
                return Some(pos..pos + needle.len());
            }
        }
    }

    #[inline]
    fn search_l(&self, buf: &[u8], _edge: bool) -> Option<Range<usize>> {
        let needle = self.delimiter.deref().as_ref();
        if needle.is_empty() {
            return None;
        }

        let b = needle[0];
        let mut pos = 0;
        loop {
            if let Some(i) = memchr(b, &buf[pos..]) {
                pos += i;
            } else {
                return None;
            }
            if buf[pos..].starts_with(needle) {
                return Some(pos..pos + needle.len());
            }
            pos += 1;
        }
    }

    #[inline]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        if self.len() < 2 {
            return None;
        }

        let end = buf.len();
        let begin = end.saturating_sub(self.len() - 1);
        (begin..end).find(|&i| self.delimiter.as_ref().starts_with(&buf[i..]))
    }

    #[inline]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        if self.len() < 2 {
            return None;
        }

        let begin = 0;
        let end = begin + min(buf.len(), self.len() - 1);
        (begin..end)
            .rev()
            .find(|&i| self.delimiter.as_ref().ends_with(&buf[..i]))
    }
}

// ---

/// Searches for a new line in a byte slice that can be either LF or CRLF.
pub struct SmartNewLineSearcher;

impl Search for SmartNewLineSearcher {
    #[inline]
    fn search_r(&self, buf: &[u8], _edge: bool) -> Option<Range<usize>> {
        memrchr(b'\n', buf).map(|i| {
            if i > 0 && buf[i - 1] == b'\r' {
                i - 1..i + 1
            } else {
                i..i + 1
            }
        })
    }

    #[inline]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        if buf.is_empty() {
            return None;
        }

        let b = if edge { 0 } else { 1 };

        memchr(b'\n', &buf[b..]).map(|i| {
            if i > 0 && buf[i - 1] == b'\r' {
                b + i - 1..b + i + 1
            } else {
                b + i..b + i + 1
            }
        })
    }

    #[inline]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        if !buf.is_empty() && buf[buf.len() - 1] == b'\r' {
            Some(buf.len() - 1)
        } else {
            None
        }
    }

    #[inline]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        if !buf.is_empty() && buf[0] == b'\n' {
            Some(1)
        } else {
            None
        }
    }
}

// ---

/// Contains a pre-allocated data buffer for a Segment and data size.
#[derive(Eq)]
pub struct SegmentBuf {
    buf: Vec<u8>,
    size: usize,
}

impl SegmentBuf {
    /// Returns a reference to the contained data.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.buf[..self.size]
    }

    /// Returns data size.
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Transforms the SegmentBuf into inner Vec<u8>.
    #[inline]
    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }

    #[inline]
    fn new(capacity: usize) -> Self {
        let buf = vec![0; capacity];
        Self { buf, size: 0 }
    }

    #[inline]
    fn zero() -> Self {
        Self {
            buf: Vec::new(),
            size: 0,
        }
    }

    #[inline]
    fn reset(&mut self) {
        self.buf.resize(self.buf.capacity(), 0);
        self.size = 0;
    }

    #[inline]
    fn resetted(mut self) -> Self {
        self.reset();
        self
    }

    #[inline]
    fn replace(&mut self, mut other: Self) -> Self {
        std::mem::swap(self, &mut other);
        other
    }
}

impl PartialEq for SegmentBuf {
    #[inline]
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
    #[inline]
    fn from(data: T) -> Self {
        let size = data.as_ref().len();
        Self {
            buf: data.as_ref().into(),
            size,
        }
    }
}

// ---

/// Segment is an output of Scanner.
/// Complete segment contains a whole number of tokens.
/// Incomplete segment contains a part of a token.
#[derive(Debug, Eq, PartialEq)]
pub enum Segment {
    Complete(SegmentBuf),
    Incomplete(SegmentBuf, PartialPlacement),
}

impl Segment {
    /// Returns a new Segment containing the given SegmentBuf.
    #[inline]
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
    buf_size: usize,
    recycled: SegQueue<SegmentBuf>,
}

impl SegmentBufFactory {
    /// Returns a new SegmentBufFactory with the given parameters.
    pub fn new(buf_size: usize) -> Self {
        Self {
            buf_size,
            recycled: SegQueue::new(),
        }
    }

    /// Returns a new or recycled SegmentBuf.
    #[inline]
    pub fn new_segment(&self) -> SegmentBuf {
        match self.recycled.pop() {
            Some(buf) => buf.resetted(),
            None => SegmentBuf::new(self.buf_size),
        }
    }

    /// Recycles the given SegmentBuf.
    #[inline]
    pub fn recycle(&self, buf: SegmentBuf) {
        if buf.buf.capacity() == self.buf_size {
            self.recycled.push(buf);
        }
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
    #[inline]
    pub fn new(buf_size: usize) -> Self {
        Self {
            buf_size,
            recycled: SegQueue::new(),
        }
    }

    /// Returns a new or recycled buffer.
    #[inline]
    pub fn new_buf(&self) -> Vec<u8> {
        match self.recycled.pop() {
            Some(mut buf) => {
                buf.clear();
                buf
            }
            None => Vec::with_capacity(self.buf_size),
        }
    }

    /// Recycles the given buffer.
    #[inline]
    pub fn recycle(&self, buf: Vec<u8>) {
        self.recycled.push(buf);
    }
}

// ---

/// Iterates over the input stream and returns segments containing one or more tokens.
pub struct ScannerIter<'a, 'b, D: Delimit> {
    scanner: &'a Scanner<D>,
    input: &'b mut dyn Read,
    next: SegmentBuf,
    searcher: D::Searcher,
    placement: Option<PartialPlacement>,
    done: bool,
}

impl<'a, 'b, D: Delimit> ScannerIter<'a, 'b, D> {
    #[inline]
    pub fn with_max_segment_size(self, max_segment_size: usize) -> ScannerJumboIter<'a, 'b, D> {
        ScannerJumboIter::new(self, max_segment_size)
    }

    #[inline]
    fn new(scanner: &'a Scanner<D>, input: &'b mut dyn Read) -> Self {
        Self {
            scanner,
            input,
            next: scanner.sf.new_segment(),
            searcher: scanner.delimiter.clone().into_searcher(),
            placement: None,
            done: false,
        }
    }

    #[inline]
    fn split(&mut self, full: bool, edge: bool) -> Option<(SegmentBuf, bool)> {
        if self.next.len() < 1 {
            return None;
        }

        let buf = self.next.data();
        let bs = buf.len();
        self.searcher
            .search_r(buf, edge)
            .map(|range| (range.end, true))
            .or_else(|| {
                if full {
                    self.searcher.partial_match_r(buf).map(|n| (n, false))
                } else {
                    None
                }
            })
            .and_then(|(n, ok)| self.split_n(bs - n).map(|sb| (sb, ok)))
    }

    #[inline]
    fn split_n(&mut self, n: usize) -> Option<SegmentBuf> {
        let bs = self.next.len();
        if n == bs {
            return None;
        }

        let mut result = self.scanner.sf.new_segment();
        if result.buf.len() < n {
            result.buf.resize(n, 0);
        }

        if n > 0 {
            result.buf[..n].copy_from_slice(&self.next.buf[bs - n..bs]);
            result.size = n;
            self.next.size -= n;
        }

        Some(result)
    }
}

impl<'a, 'b, D: Delimit> Iterator for ScannerIter<'a, 'b, D> {
    type Item = Result<Segment>;

    fn next(&mut self) -> Option<Self::Item> {
        let bs = self.next.buf.len();

        loop {
            let begin = self.next.size;
            let end = bs;
            let n = match self.input.read(&mut self.next.buf[begin..end]) {
                Ok(n) => n,
                Err(err) => {
                    self.done = true;
                    return Some(Err(err.into()));
                }
            };

            self.next.size += n;
            let full = self.next.size == end;

            let (next, placement) = if n == 0 {
                self.done = true;
                (SegmentBuf::zero(), self.placement.and(Some(PartialPlacement::Last)))
            } else {
                match self.split(full, self.done) {
                    Some((next, true)) => {
                        let result = (next, self.placement.and(Some(PartialPlacement::Last)));
                        self.placement = None;
                        result
                    }
                    Some((next, false)) => {
                        self.placement = self
                            .placement
                            .and(Some(PartialPlacement::Next))
                            .or(Some(PartialPlacement::First));
                        (next, self.placement)
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
pub struct ScannerJumboIter<'a, 'b, D: Delimit> {
    inner: ScannerIter<'a, 'b, D>,
    max_segment_size: usize,
    fetched: VecDeque<(SegmentBuf, PartialPlacement)>,
    next: Option<Result<Segment>>,
}

impl<'a, 'b, D: Delimit> ScannerJumboIter<'a, 'b, D> {
    #[inline]
    fn new(inner: ScannerIter<'a, 'b, D>, max_segment_size: usize) -> Self {
        Self {
            inner,
            max_segment_size,
            fetched: VecDeque::new(),
            next: None,
        }
    }

    #[inline]
    fn push(&mut self, buf: SegmentBuf, placement: PartialPlacement) {
        self.fetched.push_back((buf, placement));
    }

    #[inline]
    fn pop(&mut self) -> Option<(SegmentBuf, PartialPlacement)> {
        self.fetched.pop_front()
    }

    #[inline]
    fn can_complete(&self) -> bool {
        !self.fetched.is_empty()
            && self.fetched.front().map(|x| x.1) == Some(PartialPlacement::First)
            && self.fetched.back().map(|x| x.1) == Some(PartialPlacement::Last)
    }

    #[inline]
    fn complete(&mut self) -> Option<Result<Segment>> {
        let buf = self
            .fetched
            .iter()
            .flat_map(|(buf, _)| buf.data())
            .cloned()
            .collect::<Vec<u8>>();
        for (buf, _) in self.fetched.drain(..) {
            self.inner.scanner.sf.recycle(buf);
        }

        Some(Ok(Segment::Complete(buf.into())))
    }
}

impl<'a, 'b, D: Delimit> Iterator for ScannerJumboIter<'a, 'b, D> {
    type Item = Result<Segment>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((buf, placement)) = self.pop() {
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
                        total += buf.len();
                        self.push(buf, placement);
                        if self.can_complete() {
                            return self.complete();
                        }
                        if placement == PartialPlacement::Last {
                            break;
                        }
                    }
                    next @ Some(_) => {
                        self.next = next;
                        break;
                    }
                    None => {
                        if self.fetched.is_empty() && self.next.is_none() {
                            return None;
                        }
                        break;
                    }
                };
                if total >= self.max_segment_size {
                    break;
                }
            }
        }
    }
}

// ---

#[cfg(test)]
mod tests;
