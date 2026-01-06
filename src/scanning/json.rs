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

/// Searches for a whitespace boundary between two top-level JSON objects.
pub struct JsonDelimitSearcher;

impl Search for JsonDelimitSearcher {
    #[inline]
    fn search_r(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        for j in memrchr_iter(b'{', buf) {
            if let Some(i) = memrchr(b'}', &buf[..j]) {
                if valid_space(&buf[i + 1..j]) {
                    return Some(i + 1..j);
                }
            } else if edge && valid_space(&buf[..j]) {
                return Some(0..j);
            }
        }
        None
    }

    #[inline]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        for i in memchr_iter(b'}', buf) {
            if let Some(j) = memchr(b'{', &buf[i..]) {
                let j = i + j;
                if valid_space(&buf[i + 1..j]) {
                    return Some(i + 1..j);
                }
            } else if edge && valid_space(&buf[i + 1..]) {
                return Some(i + 1..buf.len());
            }
        }
        None
    }

    #[inline]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        if let Some(i) = memrchr(b'}', buf) {
            if valid_space(&buf[i + 1..]) {
                return Some(buf.len() - i);
            }
        }
        None
    }

    #[inline]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        if let Some(i) = memchr(b'{', buf) {
            if valid_space(&buf[..i]) {
                return Some(i);
            }
        }
        None
    }
}

#[inline]
fn valid_space(s: &[u8]) -> bool {
    let mut has_newlines = false;

    for &c in s {
        match c {
            b'\n' | b'\r' => has_newlines = true,
            b' ' | b'\t' => {}
            _ => return false,
        }
    }

    has_newlines
}
