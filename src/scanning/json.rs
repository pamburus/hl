// std imports
use std::ops::Range;

// third-party imports
use memchr::{memchr, memchr2_iter, memrchr, memrchr2_iter};

// relative imports
use super::{Delimit, Search, SmartNewLineSearcher};

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
        for i in memrchr2_iter(b'{', b'}', buf) {
            if let Some(range) = resolve_delimiter(buf, i, edge) {
                return Some(range);
            }
        }
        if edge {
            SmartNewLineSearcher.search_l(buf, edge)
        } else {
            None
        }
    }

    #[inline]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        for i in memchr2_iter(b'{', b'}', buf) {
            if let Some(range) = resolve_delimiter(buf, i, edge) {
                return Some(range);
            }
        }
        if edge {
            SmartNewLineSearcher.search_r(buf, edge)
        } else {
            None
        }
    }

    #[inline]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        if let Some(i) = memrchr(b'}', buf) {
            if whitespace_only(&buf[i + 1..]) {
                return Some(i + 1);
            }
        }
        None
    }

    #[inline]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        if let Some(i) = memchr(b'{', buf) {
            if whitespace_only(&buf[..i]) {
                return Some(i);
            }
        }
        None
    }
}

#[inline]
fn whitespace_only(s: &[u8]) -> bool {
    s.iter().all(|&c| matches!(c, b' ' | b'\t' | b'\n' | b'\r'))
}

#[inline]
fn find_left_boundary_begin(s: &[u8], edge: bool) -> Option<usize> {
    let mut has_newlines = false;
    for (i, &c) in s.iter().enumerate().rev() {
        match c {
            b' ' | b'\t' => continue,
            b'\n' | b'\r' => has_newlines = true,
            b',' | b'[' | b':' => return None,
            _ => return if has_newlines { Some(i + 1) } else { None },
        }
    }
    if edge { Some(0) } else { None }
}

#[inline]
fn find_right_boundary_end(s: &[u8], edge: bool) -> Option<usize> {
    let mut has_newlines = false;
    for (i, &c) in s.iter().enumerate() {
        match c {
            b' ' | b'\t' => continue,
            b'\n' | b'\r' => has_newlines = true,
            b',' | b']' => return None,
            _ => return if has_newlines { Some(i) } else { None },
        }
    }
    if edge { Some(s.len()) } else { None }
}

#[inline]
fn resolve_delimiter(buf: &[u8], i: usize, edge: bool) -> Option<Range<usize>> {
    let c = buf[i];
    match c {
        b'}' => {
            let i = i + 1;
            if i < buf.len() {
                if let Some(j) = find_right_boundary_end(&buf[i..], edge) {
                    let j = i + j;
                    return Some(i..j);
                }
            }
        }
        b'{' => {
            if i != 0 {
                if let Some(j) = find_left_boundary_begin(&buf[..i], edge) {
                    return Some(j..i);
                }
            }
        }
        _ => {}
    }
    None
}
