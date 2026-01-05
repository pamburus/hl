// std imports
use std::ops::Range;

// third-party imports
use memchr::{memchr, memchr_iter, memrchr, memrchr_iter};

// relative imports
use super::{Delimit, Search};

#[derive(Clone)]
pub struct JsonDelimiter;

impl Delimit for JsonDelimiter {
    type Searcher = JsonDelimitSearcher;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        Self::Searcher {}
    }
}

/// Searches for a new line in a byte slice that can be either LF or CRLF.
pub struct JsonDelimitSearcher;

impl Search for JsonDelimitSearcher {
    #[inline]
    fn search_r(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        for j in memrchr_iter(b'{', buf) {
            if let Some(i) = memrchr(b'}', &buf[..j]) {
                if buf[i + 1..j].iter().all(|&c| c.is_ascii_whitespace()) {
                    return Some(i + 1..j);
                }
            } else if edge {
                if buf[..j].iter().all(|&c| c.is_ascii_whitespace()) {
                    return Some(0..j);
                }
            }
        }
        None
    }

    #[inline]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        for i in memchr_iter(b'}', buf) {
            if let Some(j) = memchr(b'{', &buf[i..]) {
                let j = i + j;
                if buf[i + 1..j].iter().all(|&c| c.is_ascii_whitespace()) {
                    return Some(i + 1..j);
                }
            } else if edge {
                if buf[i + 1..].iter().all(|&c| c.is_ascii_whitespace()) {
                    return Some(i + 1..buf.len());
                }
            }
        }
        None
    }

    #[inline]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        if let Some(i) = memrchr(b'}', buf) {
            if buf[i + 1..].iter().all(|&c| c.is_ascii_whitespace()) {
                return Some(buf.len() - i);
            }
        }
        None
    }

    #[inline]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        if let Some(i) = memchr(b'{', buf) {
            if buf[..i].iter().all(|&c| c.is_ascii_whitespace()) {
                return Some(i);
            }
        }
        None
    }
}
