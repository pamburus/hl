//! Substring + hl::Query search over a byte range, scanned line-by-line through the
//! shared rendering pipeline.
//!
//! Two modes:
//!
//! - **Substring**: the chunk is rendered to ANSI bytes, ANSI codes are stripped to
//!   produce the plain text the user actually sees, and we scan for occurrences of
//!   the needle inside that plain text. Returns char offsets per match so the client
//!   can highlight. Case-insensitive search uses ASCII case folding so match
//!   positions don't shift relative to the original text.
//! - **Query**: the needle is parsed as an `hl::Query` expression and used directly
//!   as the [`RecordFilter`] passed to the rendering pipeline. Lines whose record
//!   passes the filter are reported (no char ranges — the whole record matched).
//!
//! Both modes scan a single byte range per call. Clients paginate.

use serde::Serialize;
use thiserror::Error;

use crate::render::RenderConfig;

/// Which search mode to use. Stable over the wire; `mode=substring` or `mode=query`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    #[default]
    Substring,
    Query,
}

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("invalid query: {0}")]
    InvalidQuery(String),
}

/// One matched line.
#[derive(Debug, Serialize)]
pub struct SearchMatch {
    /// Byte offset of the matched line's first byte in the source file.
    pub start: u64,
    /// For substring mode: `[start_char, end_char]` ranges inside the line's plain
    /// text, in Unicode code-point units. Empty for query-mode matches (the whole
    /// record matched).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub ranges: Vec<[u32; 2]>,
}

/// Scan `bytes` (the byte range `[first_byte, first_byte + bytes.len())` of the
/// source) for matches according to `mode`. Empty needle returns no matches.
pub fn search_chunk(
    config: &RenderConfig,
    bytes: &[u8],
    first_byte: u64,
    needle: &str,
    mode: SearchMode,
    case_insensitive: bool,
) -> Result<Vec<SearchMatch>, SearchError> {
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    match mode {
        SearchMode::Substring => Ok(substring_search(config, bytes, first_byte, needle, case_insensitive)),
        SearchMode::Query => {
            let query = hl::Query::parse(needle).map_err(|e| SearchError::InvalidQuery(format!("{e}")))?;
            Ok(query_search(config, bytes, first_byte, query))
        }
    }
}

fn substring_search(
    config: &RenderConfig,
    bytes: &[u8],
    first_byte: u64,
    needle: &str,
    case_insensitive: bool,
) -> Vec<SearchMatch> {
    let needle_bytes = needle.as_bytes();
    let needle_folded: Vec<u8>;
    let needle_match: &[u8] = if case_insensitive {
        needle_folded = needle_bytes.iter().map(u8::to_ascii_lowercase).collect();
        &needle_folded
    } else {
        needle_bytes
    };

    let mut renderer = config.renderer();
    let mut matches = Vec::new();
    let mut plain = Vec::new();
    let mut hay_folded = Vec::new();
    renderer.render_chunk(bytes, first_byte, hl::Filter::default(), |r| {
        if r.ansi.is_empty() {
            return;
        }
        plain.clear();
        ansi_strip_into(r.ansi, &mut plain);

        let hay: &[u8] = if case_insensitive {
            hay_folded.clear();
            hay_folded.extend(plain.iter().map(|b| b.to_ascii_lowercase()));
            &hay_folded
        } else {
            &plain
        };

        let mut ranges: Vec<[u32; 2]> = Vec::new();
        let mut p = 0;
        while let Some(idx) = memmem(&hay[p..], needle_match) {
            let byte_start = p + idx;
            let byte_end = byte_start + needle_match.len();
            // Convert byte positions in `plain` to char (Unicode code-point) offsets so
            // the client can highlight without needing to know UTF-8 layout.
            let start_chars = chars_up_to(&plain, byte_start);
            let end_chars = chars_up_to(&plain, byte_end);
            ranges.push([start_chars, end_chars]);
            p = byte_end.max(byte_start + 1);
        }
        if !ranges.is_empty() {
            matches.push(SearchMatch { start: r.start, ranges });
        }
    });
    matches
}

fn query_search(config: &RenderConfig, bytes: &[u8], first_byte: u64, query: hl::Query) -> Vec<SearchMatch> {
    let mut renderer = config.renderer();
    let mut matches = Vec::new();
    renderer.render_chunk(bytes, first_byte, query, |r| {
        // Sink fires for every input line; only lines that passed the filter
        // produce non-empty `ansi`. Recording offsets only for those = the
        // matching records.
        if !r.ansi.is_empty() {
            matches.push(SearchMatch {
                start: r.start,
                ranges: Vec::new(),
            });
        }
    });
    matches
}

/// Drop ANSI CSI escape sequences from `bytes`, appending the surviving payload to
/// `out`. Non-CSI escapes (cursor moves, OSC, etc.) are dropped silently — same rule
/// the segmenter follows.
fn ansi_strip_into(bytes: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1B && bytes.get(i + 1) == Some(&b'[') {
            // CSI: parameters (0x30–0x3F) / intermediates (0x20–0x2F) / final (0x40–0x7E).
            let mut j = i + 2;
            while j < bytes.len() && bytes[j] >= 0x20 && bytes[j] < 0x40 {
                j += 1;
            }
            i = if j < bytes.len() { j + 1 } else { bytes.len() };
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
}

/// Byte-level substring search using `memchr` when the needle's first byte is
/// distinctive, falling back to naive scan. Avoids pulling in the `memchr` crate
/// directly — the workspace already has it as a transitive dep, and `Vec::windows`
/// is fine at the sizes we deal with per chunk.
fn memmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Count Unicode code points in `bytes[..byte_pos]`. Falls back to raw byte count on
/// invalid UTF-8 so we never panic on malformed input.
fn chars_up_to(bytes: &[u8], byte_pos: usize) -> u32 {
    let pos = byte_pos.min(bytes.len());
    match std::str::from_utf8(&bytes[..pos]) {
        Ok(s) => s.chars().count() as u32,
        Err(_) => pos as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_sgr_sequences() {
        let mut out = Vec::new();
        ansi_strip_into(b"\x1b[1;31mred\x1b[0m plain", &mut out);
        assert_eq!(out, b"red plain");
    }

    #[test]
    fn strips_non_sgr_csi() {
        let mut out = Vec::new();
        ansi_strip_into(b"\x1b[H\x1b[2Jhello", &mut out);
        assert_eq!(out, b"hello");
    }

    #[test]
    fn memmem_finds_needle() {
        assert_eq!(memmem(b"abcdef", b"cde"), Some(2));
        assert_eq!(memmem(b"abcdef", b"xyz"), None);
        assert_eq!(memmem(b"abc", b""), None);
        assert_eq!(memmem(b"", b"abc"), None);
    }

    #[test]
    fn chars_up_to_handles_multibyte() {
        // "héllo" — é is 2 bytes in UTF-8.
        let s = "héllo".as_bytes();
        assert_eq!(chars_up_to(s, 0), 0);
        assert_eq!(chars_up_to(s, 1), 1);
        assert_eq!(chars_up_to(s, 3), 2); // through "hé"
        assert_eq!(chars_up_to(s, s.len()), 5);
    }
}
