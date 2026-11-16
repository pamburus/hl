// std imports
use std::ops::Range;

// relative imports
use super::{Delimit, NewlineSearcher, Search, SearchProgress};

#[derive(Clone)]
pub struct PrettyCompatibleDelimiter;

impl Delimit for PrettyCompatibleDelimiter {
    type Searcher = PrettyCompatibleSearcher;

    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        Self::Searcher {}
    }
}

/// Searches for a new line in a byte slice that can be either LF or CRLF
/// surrounded by lines starting with non-whitespace characters.
pub struct PrettyCompatibleSearcher;

impl PrettyCompatibleSearcher {
    /// Searches for the last delimiter in the buffer with its line feed at `from` or later.
    #[inline]
    fn search_r_from(&self, buf: &[u8], edge: bool, from: usize) -> Option<Range<usize>> {
        let mut r = buf.len();
        while let Some(range) = NewlineSearcher::search_r_from(&buf[..r], from.min(r)) {
            if edge && range.end == buf.len() {
                return Some(range);
            }

            match buf.get(range.end) {
                Some(b'}' | b' ' | b'\t') => {}
                _ => return Some(range),
            }

            r = range.start;
        }
        None
    }
}

impl Search for PrettyCompatibleSearcher {
    #[inline]
    fn search_r(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        self.search_r_from(buf, edge, 0)
    }

    #[inline]
    fn search_r_incremental(&self, buf: &[u8], progress: &mut SearchProgress) -> Option<Range<usize>> {
        let found = self.search_r_from(buf, true, progress.from);
        if found.is_none() {
            // A new line in the searched data is either followed there by a character that keeps it from being a delimiter,
            // or it ends the searched data and would have been found as a delimiter at the edge.
            *progress = SearchProgress::up_to(buf, buf.len());
        }
        found
    }

    #[inline]
    fn search_l(&self, buf: &[u8], edge: bool) -> Option<Range<usize>> {
        let mut l = 0;
        while let Some(range) = NewlineSearcher.search_l(&buf[l..], edge) {
            let range = (l + range.start)..(l + range.end);

            if edge && range.start == 0 {
                return Some(range);
            }

            match buf.get(range.end) {
                Some(b'}' | b' ' | b'\t') => {}
                _ => return Some(range),
            }

            l = range.end;
        }
        None
    }

    #[inline]
    fn partial_match_r(&self, buf: &[u8]) -> Option<usize> {
        if let Some(m) = NewlineSearcher.partial_match_r(buf) {
            return Some(m);
        }
        if let Some(&b'\n') = buf.last() {
            if buf.len() >= 2 && buf[buf.len() - 2] == b'\r' {
                return Some(buf.len() - 2);
            }
            return Some(buf.len() - 1);
        }
        None
    }

    #[inline]
    fn partial_match_l(&self, buf: &[u8]) -> Option<usize> {
        NewlineSearcher.partial_match_l(buf)
    }
}

#[cfg(test)]
mod tests;
