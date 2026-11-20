// std imports
use std::cmp::min;
use std::collections::VecDeque;
use std::convert::From;
use std::io::Read;
use std::ops::{Deref, Range};
use std::sync::Arc;

// third-party imports
use memchr::{memchr, memrchr};
use serde::{Deserialize, Serialize};

// local imports
use crate::error::*;
use crate::pool::{Pool, Pooled, Recycler};

// ---

mod auto;
mod json;

pub use auto::PrettyCompatibleDelimiter;
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
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Delimiter {
    #[default]
    PrettyCompatible,
    Byte(u8),
    Bytes(Arc<[u8]>),
    Char(char),
    Str(Arc<str>),
    Newline,
    Json,
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

impl From<Newline> for Delimiter {
    #[inline]
    fn from(_: Newline) -> Self {
        Self::Newline
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
            Self::Newline => Arc::new(Newline.into_searcher()),
            Self::Json => Arc::new(JsonDelimiter.into_searcher()),
            Self::PrettyCompatible => Arc::new(PrettyCompatibleDelimiter.into_searcher()),
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
pub struct Newline;

impl Delimit for Newline {
    type Searcher = NewlineSearcher;

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

    /// Searches for the delimiter in the buffer from the right, like [`Search::search_r`] with `edge`,
    /// in a buffer that only grows at its end between searches.
    ///
    /// The first search of a buffer starts from [`SearchProgress::default`]. A search that finds no delimiter
    /// updates `progress` with what the next search of the grown buffer needs to know, so that only
    /// the delimiters that the grown data can complete are looked for, and searching a growing buffer
    /// each time it grows costs no more than its length in total. After a delimiter is found, or the
    /// buffer changes before its end, the next search starts from [`SearchProgress::default`] again.
    #[must_use]
    fn search_r_incremental(&self, buf: &[u8], progress: &mut SearchProgress) -> Option<Range<usize>>;

    /// Searches for the delimiter in the buffer from the left.
    #[must_use]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>>;

    /// Searches for a partial match of the delimiter at the right edge of the buffer.
    ///
    /// Returns the position (index from the beginning of the buffer) where a potential
    /// partial delimiter match starts at the end of the buffer.
    ///
    /// For example, if delimiter is "abc" and buffer is "xyzab", this returns Some(3)
    /// because "ab" at position 3 could be the start of "abc".
    ///
    /// Used in Scanner as: `bs - position` to calculate the length of bytes to extract.
    #[must_use]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize>;

    /// Searches for a partial match of the delimiter at the left edge of the buffer.
    ///
    /// Returns the position (index from the beginning of the buffer) where a potential
    /// partial delimiter match ends at the start of the buffer.
    ///
    /// For example, if delimiter is "abc" and buffer is "bcxyz", this returns Some(2)
    /// because "bc" at buf[0..2] could be the end of "abc".
    ///
    /// Since the match starts at position 0, the returned position also equals the length
    /// of the partial match.
    #[must_use]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize>;
}

/// What a search of a growing buffer that found no delimiter knows for the next search,
/// see [`Search::search_r_incremental`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchProgress {
    // `buf[..len]` has been searched.
    len: usize,
    // What the searcher keeps for the next search, which for most of them is where it starts,
    // as a delimiter can only be completed with the data at `from` or later. The JSON searcher
    // starts at `len` instead and keeps here a closing brace it has yet to resolve, if `from < len`.
    from: usize,
}

impl SearchProgress {
    /// Returns where the searched data ends.
    #[inline]
    pub fn end(&self) -> usize {
        self.len
    }

    /// Returns what the next search of `buf` needs to know when a delimiter can only be completed with the data at `from` or later.
    #[inline]
    fn up_to(buf: &[u8], from: usize) -> Self {
        Self { len: buf.len(), from }
    }
}

impl Search for u8 {
    #[inline(always)]
    fn search_r(&self, buf: &[u8], _: bool) -> Option<Range<usize>> {
        memrchr(*self, buf).map(|x| x..x + 1)
    }

    #[inline(always)]
    fn search_r_incremental(&self, buf: &[u8], progress: &mut SearchProgress) -> Option<Range<usize>> {
        let from = progress.from;
        let found = memrchr(*self, &buf[from..]).map(|x| from + x..from + x + 1);
        if found.is_none() {
            *progress = SearchProgress::up_to(buf, buf.len());
        }
        found
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
    fn search_r_incremental(&self, buf: &[u8], progress: &mut SearchProgress) -> Option<Range<usize>> {
        self.as_ref().search_r_incremental(buf, progress)
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
    fn search_r_incremental(&self, buf: &[u8], progress: &mut SearchProgress) -> Option<Range<usize>> {
        self.as_ref().search_r_incremental(buf, progress)
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

    /// Searches for the last delimiter in the buffer that starts at `from` or later.
    #[inline]
    fn search_r_from(&self, buf: &[u8], from: usize) -> Option<Range<usize>> {
        let needle = self.delimiter.deref().as_ref();
        if needle.is_empty() {
            return None;
        }

        let b = needle[0];
        let mut pos = buf.len();
        loop {
            if let Some(i) = memrchr(b, &buf[from..pos]) {
                pos = from + i;
            } else {
                return None;
            }
            if buf[pos..].starts_with(needle) {
                return Some(pos..pos + needle.len());
            }
        }
    }
}

impl<D> Search for SubStrSearcher<D>
where
    D: Deref,
    D::Target: AsRef<[u8]>,
{
    #[inline]
    fn search_r(&self, buf: &[u8], _edge: bool) -> Option<Range<usize>> {
        self.search_r_from(buf, 0)
    }

    #[inline]
    fn search_r_incremental(&self, buf: &[u8], progress: &mut SearchProgress) -> Option<Range<usize>> {
        let found = self.search_r_from(buf, progress.from);
        if found.is_none() {
            // A delimiter that fits in the searched data would have been found there.
            *progress = SearchProgress::up_to(buf, (buf.len() + 1).saturating_sub(self.len()).min(buf.len()));
        }
        found
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
pub struct NewlineSearcher;

impl NewlineSearcher {
    /// Searches for the last new line in the buffer with its line feed at `from` or later.
    #[inline]
    fn search_r_from(buf: &[u8], from: usize) -> Option<Range<usize>> {
        memrchr(b'\n', &buf[from..]).map(|i| {
            let i = from + i;
            if i > 0 && buf[i - 1] == b'\r' {
                i - 1..i + 1
            } else {
                i..i + 1
            }
        })
    }
}

impl Search for NewlineSearcher {
    #[inline]
    fn search_r(&self, buf: &[u8], _edge: bool) -> Option<Range<usize>> {
        Self::search_r_from(buf, 0)
    }

    #[inline]
    fn search_r_incremental(&self, buf: &[u8], progress: &mut SearchProgress) -> Option<Range<usize>> {
        let found = Self::search_r_from(buf, progress.from);
        if found.is_none() {
            // A line feed in the searched data would have been found there.
            *progress = SearchProgress::up_to(buf, buf.len());
        }
        found
    }

    #[inline]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        if buf.is_empty() {
            return None;
        }

        let b = if edge { 0 } else { 1 };

        memchr(b'\n', &buf[b..]).map(|i| {
            let i = b + i;
            if i > 0 && buf[i - 1] == b'\r' {
                i - 1..i + 1
            } else {
                i..i + 1
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
///
/// A buffer taken from a [`SegmentBufFactory`] returns to it when dropped.
pub struct SegmentBuf {
    buf: Pooled<SegmentBufRecycler>,
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

    /// Returns the size of the buffer, which the data may not fill.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Shortens the data to the given length, keeping the buffer.
    /// Has no effect if the data is already shorter.
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        self.size = self.size.min(len);
    }

    /// Appends the given data, growing the buffer only if it does not fit.
    #[inline]
    pub fn extend_from_slice(&mut self, data: &[u8]) {
        let end = self.size + data.len();
        if end > self.buf.len() {
            // Growing it by the default amortized policy could double the buffer for a few bytes.
            // The length is read first, as two-phase borrows do not reach through `DerefMut`.
            let additional = end - self.buf.len();
            self.buf.reserve_exact(additional);
            self.buf.resize(end, 0);
        }
        self.buf[self.size..end].copy_from_slice(data);
        self.size = end;
    }

    /// Returns a SegmentBuf holding the given data without copying it, in a buffer that does not belong to any pool.
    #[inline]
    pub fn from_vec(data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            buf: Pooled::detached(data),
            size,
        }
    }

    /// Transforms the SegmentBuf into inner Vec<u8> holding only the data.
    /// The buffer no longer returns to its pool.
    #[inline]
    pub fn into_data(self) -> Vec<u8> {
        let size = self.size;
        let mut buf = self.buf.into_inner();
        buf.truncate(size);
        buf
    }

    #[inline]
    fn zero() -> Self {
        Self {
            buf: Pooled::detached(Vec::new()),
            size: 0,
        }
    }

    #[inline]
    fn replace(&mut self, mut other: Self) -> Self {
        std::mem::swap(self, &mut other);
        other
    }
}

impl Eq for SegmentBuf {}

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
    /// Returns a SegmentBuf holding a copy of the given data, in a buffer that does not belong to any pool.
    #[inline]
    fn from(data: T) -> Self {
        let size = data.as_ref().len();
        Self {
            buf: Pooled::detached(data.as_ref().into()),
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
    /// Also holds the length of the delimiter the part ends with, zero unless it is the last part,
    /// and for the last part if the input ended without one.
    Incomplete(SegmentBuf, PartialPlacement, usize),
}

impl Segment {
    /// Returns the data of the segment, whether it is complete or not.
    #[inline]
    pub fn data(&self) -> &[u8] {
        match self {
            Self::Complete(buf) | Self::Incomplete(buf, ..) => buf.data(),
        }
    }

    /// Returns a new Segment containing the given SegmentBuf.
    #[inline]
    fn new(buf: SegmentBuf, placement: Option<PartialPlacement>, delimiter: usize) -> Self {
        if let Some(placement) = placement {
            Self::Incomplete(buf, placement, delimiter)
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

/// Constructs new SegmentBuf's with the configured size and takes back the ones dropped, see [`SegmentBufRecycler`].
pub type SegmentBufFactory = Pool<SegmentBufRecycler>;

impl SegmentBufFactory {
    /// Returns a new SegmentBufFactory with the given parameters.
    #[inline]
    pub fn new(buf_size: usize) -> Self {
        Self::with_recycler(SegmentBufRecycler { buf_size })
    }

    /// Returns a new or recycled SegmentBuf, which returns to the factory when dropped.
    #[inline]
    pub fn new_segment(self: &Arc<Self>) -> SegmentBuf {
        SegmentBuf {
            buf: self.take(),
            size: 0,
        }
    }
}

/// Creates the buffers of a [`SegmentBufFactory`] and decides which returned ones it keeps.
///
/// Only buffers of exactly the configured size are kept, as the scanner reads into the whole buffer.
pub struct SegmentBufRecycler {
    buf_size: usize,
}

impl Recycler for SegmentBufRecycler {
    type Item = Vec<u8>;

    #[inline]
    fn create(&self) -> Vec<u8> {
        vec![0; self.buf_size]
    }

    #[inline]
    fn reuse(&self, buf: &mut Vec<u8>) {
        buf.resize(buf.capacity(), 0);
    }

    #[inline]
    fn keep(&self, buf: &Vec<u8>) -> bool {
        buf.capacity() == self.buf_size
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
    pending: bool,
    done: bool,
    open_ended: bool,
}

impl<'a, 'b, D: Delimit> ScannerIter<'a, 'b, D> {
    #[inline]
    pub fn with_max_segment_size(self, max_segment_size: usize) -> ScannerJumboIter<'a, 'b, D> {
        ScannerJumboIter::new(self, max_segment_size)
    }

    /// Continues a token that a previous input left incomplete, see [`ScannerIter::open_ended`]:
    /// the data up to the first delimiter is its rest, and none of its parts is marked as the first one.
    #[inline]
    pub fn continuing(mut self, continuing: bool) -> Self {
        if continuing {
            self.placement = Some(PartialPlacement::Next);
        }
        self
    }

    /// Leaves a token split into parts that is incomplete at the end of the input open, to be continued by another input,
    /// instead of marking its last part as [`PartialPlacement::Last`].
    #[inline]
    pub fn open_ended(mut self, open_ended: bool) -> Self {
        self.open_ended = open_ended;
        self
    }

    #[inline]
    fn new(scanner: &'a Scanner<D>, input: &'b mut dyn Read) -> Self {
        Self {
            scanner,
            input,
            next: scanner.sf.new_segment(),
            searcher: scanner.delimiter.clone().into_searcher(),
            placement: None,
            pending: false,
            done: false,
            open_ended: false,
        }
    }

    /// Splits off the data after the last token, or after an incomplete token,
    /// returning it along with the length of the delimiter it was split at, if any.
    #[inline]
    fn split(&mut self, full: bool, edge: bool) -> Option<(SegmentBuf, Option<usize>)> {
        if self.next.len() < 1 {
            return None;
        }

        let buf = self.next.data();
        let bs = buf.len();
        // An incomplete token ends at the first delimiter, and the tokens after it start a new segment.
        let found = if self.placement.is_some() {
            self.searcher.search_l(buf, true)
        } else {
            self.searcher.search_r(buf, edge)
        };
        found
            .map(|range| (range.end, Some(range.len())))
            .or_else(|| {
                if full {
                    self.searcher.partial_match_r(buf).map(|n| (n, None))
                } else {
                    None
                }
            })
            .and_then(|(n, delimiter)| self.split_n(bs - n).map(|sb| (sb, delimiter)))
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
            // Tokens left after an incomplete token are split before reading more input.
            let n = if std::mem::take(&mut self.pending) {
                None
            } else {
                match self.input.read(&mut self.next.buf[begin..end]) {
                    Ok(n) => Some(n),
                    Err(err) => {
                        self.done = true;
                        return Some(Err(err.into()));
                    }
                }
            };

            self.next.size += n.unwrap_or(0);
            let full = self.next.size == end;

            let (next, placement, delimiter) = if n == Some(0) {
                self.done = true;
                let end = if self.open_ended {
                    PartialPlacement::Next
                } else {
                    PartialPlacement::Last
                };
                (SegmentBuf::zero(), self.placement.take().and(Some(end)), 0)
            } else {
                match self.split(full, self.done) {
                    Some((next, Some(delimiter))) => {
                        self.pending = self.placement.is_some() && next.len() != 0;
                        let result = (next, self.placement.and(Some(PartialPlacement::Last)), delimiter);
                        self.placement = None;
                        result
                    }
                    Some((next, None)) => {
                        self.placement = self
                            .placement
                            .and(Some(PartialPlacement::Next))
                            .or(Some(PartialPlacement::First));
                        (next, self.placement, 0)
                    }
                    None => {
                        if !full {
                            continue;
                        }
                        self.placement = self
                            .placement
                            .and(Some(PartialPlacement::Next))
                            .or(Some(PartialPlacement::First));
                        (self.scanner.sf.new_segment(), self.placement, 0)
                    }
                }
            };

            let result = self.next.replace(next);
            // A token split into parts always ends with its last part, empty if the input ended right after a whole part.
            return if result.size != 0 || placement == Some(PartialPlacement::Last) {
                Some(Ok(Segment::new(result, placement, delimiter)))
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
    fetched: VecDeque<(SegmentBuf, PartialPlacement, usize)>,
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
    fn push(&mut self, buf: SegmentBuf, placement: PartialPlacement, delimiter: usize) {
        self.fetched.push_back((buf, placement, delimiter));
    }

    #[inline]
    fn pop(&mut self) -> Option<(SegmentBuf, PartialPlacement, usize)> {
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
        // One allocation of the exact size, filled part by part.
        let total = self.fetched.iter().map(|(buf, ..)| buf.len()).sum();
        let mut buf = Vec::with_capacity(total);
        for (part, ..) in &self.fetched {
            buf.extend_from_slice(part.data());
        }
        // The parts return to their pool.
        self.fetched.clear();

        // Moved in, not copied again as `buf.into()` would.
        Some(Ok(Segment::Complete(SegmentBuf::from_vec(buf))))
    }
}

impl<'a, 'b, D: Delimit> Iterator for ScannerJumboIter<'a, 'b, D> {
    type Item = Result<Segment>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((buf, placement, delimiter)) = self.pop() {
                return Some(Ok(Segment::Incomplete(buf, placement, delimiter)));
            }
            if let Some(next) = self.next.take() {
                return Some(next);
            }

            let mut total = 0;

            loop {
                let next = self.inner.next();
                match next {
                    Some(Ok(Segment::Incomplete(buf, placement, delimiter))) => {
                        total += buf.len();
                        self.push(buf, placement, delimiter);
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
                    // A token exactly as large as the limit still fits if the input ends right after it.
                    if total == self.max_segment_size {
                        match self.inner.next() {
                            Some(Ok(Segment::Incomplete(buf, PartialPlacement::Last, delimiter))) if buf.len() == 0 => {
                                self.push(buf, PartialPlacement::Last, delimiter);
                                if self.can_complete() {
                                    return self.complete();
                                }
                            }
                            next => self.next = next,
                        }
                    }
                    break;
                }
            }
        }
    }
}

// ---

#[cfg(test)]
mod tests;
