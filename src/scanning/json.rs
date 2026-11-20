// std imports
use std::ops::Range;

// third-party imports
use memchr::{memchr, memchr2_iter, memrchr, memrchr2_iter};

// relative imports
use super::{Delimit, Search, SearchProgress};

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
        search_r_from(buf, edge, 0)
    }

    /// A brace in the searched data can only become a part of a delimiter if it is a closing one followed
    /// there by nothing but whitespace, and any other was resolved the same way there. Such a brace is
    /// kept in `progress`, at `from` before `len`, and is resolved again from where the searched data ends,
    /// as the whitespace after it up to there has no line feed and does not change how it is resolved.
    #[inline]
    fn search_r_incremental(&self, buf: &[u8], progress: &mut SearchProgress) -> Option<Range<usize>> {
        let SearchProgress { len, from } = *progress;
        let found = search_r_from(buf, true, len).or_else(|| {
            if from < len {
                find_right_boundary_end(&buf[len..], true).map(|j| from + 1..len + j)
            } else {
                None
            }
        });
        if found.is_none() {
            let blank = buf[len..].iter().rev().take_while(|&&c| is_whitespace(c)).count();
            let open = if blank < buf.len() - len {
                Some(buf.len() - blank - 1).filter(|&i| buf[i] == b'}')
            } else {
                Some(from).filter(|&i| i < len)
            };
            *progress = SearchProgress::up_to(buf, open.unwrap_or(buf.len()));
        }
        found
    }

    #[inline]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        for i in memchr2_iter(b'{', b'}', buf) {
            if let Some(range) = resolve_delimiter(buf, i, edge) {
                return Some(range);
            }
        }
        None
    }

    #[inline]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        if let Some(i) = memrchr(b'}', buf)
            && whitespace_only(&buf[i + 1..])
        {
            return Some(i + 1);
        }
        None
    }

    #[inline]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        if let Some(i) = memchr(b'{', buf)
            && whitespace_only(&buf[..i])
        {
            return Some(i);
        }
        None
    }
}

/// Searches for the last delimiter in the buffer that one of the braces at `from` or later belongs to.
#[inline]
fn search_r_from(buf: &[u8], edge: bool, from: usize) -> Option<Range<usize>> {
    for i in memrchr2_iter(b'{', b'}', &buf[from..]) {
        if let Some(range) = resolve_delimiter(buf, from + i, edge) {
            return Some(range);
        }
    }
    None
}

#[inline]
fn is_whitespace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

#[inline]
fn whitespace_only(s: &[u8]) -> bool {
    s.iter().all(|&c| is_whitespace(c))
}

#[inline]
fn find_left_boundary_begin(s: &[u8], edge: bool) -> Option<usize> {
    let mut first_newline = None;
    for (i, &c) in s.iter().enumerate().rev() {
        match c {
            b' ' | b'\t' => {}
            b'\n' => {
                if first_newline.is_none() {
                    first_newline = Some(i)
                }
            }
            b'\r' => {
                if first_newline == Some(i + 1) {
                    first_newline = Some(i);
                }
            }
            b',' | b'[' | b':' => return None,
            _ => return first_newline,
        }
    }
    if edge { first_newline } else { None }
}

#[inline]
fn find_right_boundary_end(s: &[u8], edge: bool) -> Option<usize> {
    let mut first_newline = None;
    for (i, &c) in s.iter().enumerate() {
        match c {
            b' ' | b'\t' | b'\r' => {}
            b'\n' => {
                if first_newline.is_none() {
                    first_newline = Some(i + 1);
                }
            }
            b',' | b'[' | b']' | b'}' => return None,
            _ => return first_newline,
        }
    }
    if edge { first_newline } else { None }
}

#[inline]
fn resolve_delimiter(buf: &[u8], i: usize, edge: bool) -> Option<Range<usize>> {
    let c = buf[i];
    match c {
        b'}' => {
            let i = i + 1;
            if i < buf.len()
                && let Some(j) = find_right_boundary_end(&buf[i..], edge)
            {
                let j = i + j;
                return Some(i..j);
            }
        }
        b'{' => {
            if i != 0
                && let Some(j) = find_left_boundary_begin(&buf[..i], edge)
            {
                return Some(j..i);
            }
        }
        _ => unreachable!(),
    }
    None
}

#[cfg(test)]
mod tests;
