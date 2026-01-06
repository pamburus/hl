// std imports
use std::ops::Range;

// relative imports
use super::{Delimit, Search, SmartNewLineSearcher};

#[derive(Clone)]
pub struct AutoDelimiter;

impl Delimit for AutoDelimiter {
    type Searcher = AutoDelimitSearcher;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        Self::Searcher {}
    }
}

/// Searches for a new line in a byte slice that can be either LF or CRLF
/// surrounded by lines starting with non-whitespace characters.
pub struct AutoDelimitSearcher;

impl Search for AutoDelimitSearcher {
    #[inline]
    fn search_r(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        let mut r = buf.len();
        while let Some(range) = SmartNewLineSearcher.search_r(&buf[..r], edge) {
            if range.start == 0 && edge {
                return Some(range);
            }

            match buf.get(range.end) {
                Some(b'}' | b' ' | b'\t') | None => {}
                _ => return Some(range),
            }

            r = range.start;
        }
        None
    }

    #[inline]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        let mut l = 0;
        while let Some(range) = SmartNewLineSearcher.search_l(&buf[l..], edge) {
            let range = (l + range.start)..(l + range.end);

            if range.end >= buf.len() && edge {
                return Some(range);
            }

            match buf.get(range.end) {
                Some(b'}' | b' ' | b'\t') | None => {}
                _ => return Some(range),
            }

            l = range.end;
        }
        None
    }

    #[inline]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        if let Some(m) = SmartNewLineSearcher.partial_match_r(buf) {
            return Some(m);
        }
        if let Some(&b'\n') = buf.last() {
            if buf.len() >= 2 && buf[buf.len() - 2] == b'\r' {
                return Some(2);
            }
            return Some(1);
        }
        None
    }

    #[inline]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        SmartNewLineSearcher.partial_match_l(buf)
    }
}
