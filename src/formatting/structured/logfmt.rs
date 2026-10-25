//! Logfmt encoding of structured output.

// std imports
use std::str;

// workspace imports
use encstr::{AnyEncodedString, EncodedString};
use serde_logfmt::logfmt::{is_bare_key, is_bare_value};

// relative imports
use super::{
    Encoder, NumberSource,
    json::{self, JsonEncoder},
};
use crate::model::RawValue;

// ---

/// Encodes records as lines of `key=value` pairs separated by single spaces.
///
/// Objects and arrays in values are written as compact JSON in a quoted string.
#[derive(Clone, Copy, Default)]
pub struct LogfmtEncoder {
    /// Whether the next pair is the first one in the record.
    first: bool,
    /// Nesting depth inside a value written as JSON, or zero outside of it.
    depth: usize,
    /// Position in the buffer where the value written as JSON begins.
    begin: usize,
    /// Encoder of the value written as JSON.
    json: JsonEncoder,
}

impl LogfmtEncoder {
    #[inline]
    fn nested(&self) -> bool {
        self.depth != 0
    }

    #[inline]
    fn begin_nested(&mut self, buf: &[u8]) {
        if self.depth == 0 {
            self.begin = buf.len();
        }
        self.depth += 1;
    }

    #[inline]
    fn end_nested(&mut self, buf: &mut Vec<u8>) {
        self.depth -= 1;
        if self.depth == 0 {
            quote_in_place(buf, self.begin);
        }
    }
}

impl Encoder for LogfmtEncoder {
    #[inline]
    fn begin_record(&mut self, _: &mut Vec<u8>) {
        *self = Self {
            first: true,
            ..Self::default()
        };
    }

    #[inline]
    fn end_record(&mut self, _: &mut Vec<u8>) {}

    #[inline]
    fn key(&mut self, buf: &mut Vec<u8>, key: &str) {
        if self.nested() {
            return self.json.key(buf, key);
        }
        if !self.first {
            buf.push(b' ');
        }
        self.first = false;
        if is_bare_key(key) {
            buf.extend(key.as_bytes());
        } else {
            json::push_str(buf, key);
        }
        buf.push(b'=');
    }

    #[inline]
    fn item(&mut self, buf: &mut Vec<u8>) {
        self.json.item(buf);
    }

    #[inline]
    fn string(&mut self, buf: &mut Vec<u8>, value: EncodedString) {
        if self.nested() {
            return self.json.string(buf, value);
        }
        let json = match value {
            EncodedString::Raw(raw) => {
                let text = raw.as_str();
                return if is_bare_str(text) {
                    buf.extend(text.as_bytes())
                } else {
                    json::push_str(buf, text)
                };
            }
            EncodedString::Json(json) => json,
        };

        // The source of a JSON string has no control characters, which the parsers reject, and no
        // unescaped quotes. So a space or `=` is all that needs quoting, and a backslash needs
        // decoding. A single scan stops at the first of them. Only after one that needs quoting
        // is the rest searched for escapes, to know whether to decode.
        let source = json.source();
        let text = &source[1..source.len() - 1];
        match memchr::memchr3(b' ', b'=', b'\\', text.as_bytes()) {
            None if !text.is_empty() && reads_as_string(text) => buf.extend(text.as_bytes()),
            None => json::push_json_str(buf, json, false),
            Some(i) if text.as_bytes()[i] != b'\\' => {
                let escaped = memchr::memchr(b'\\', &text.as_bytes()[i + 1..]).is_some();
                json::push_json_str(buf, json, escaped);
            }
            Some(_) => {
                let begin = buf.len();
                // Strings in parsed records are terminated, so decoding can't fail half way.
                _ = json.decode_lossy(&mut *buf);
                if !is_bare(&buf[begin..]) {
                    buf.truncate(begin);
                    json::push_json_str(buf, json, true);
                }
            }
        }
    }

    #[inline]
    fn bare_string(&mut self, buf: &mut Vec<u8>, value: &str) {
        if self.nested() || value.is_empty() {
            return self.string(buf, EncodedString::raw(value));
        }
        debug_assert!(is_bare_str(value), "{value:?}");
        buf.extend(value.as_bytes());
    }

    #[inline]
    fn text(&mut self, buf: &mut Vec<u8>, parts: &[&str]) {
        if self.nested() {
            return self.json.text(buf, parts);
        }
        let begin = buf.len();
        for part in parts {
            buf.extend(part.as_bytes());
        }
        if !is_bare(&buf[begin..]) {
            buf.truncate(begin);
            self.json.text(buf, parts);
        }
    }

    #[inline]
    fn number(&mut self, buf: &mut Vec<u8>, value: &str, source: NumberSource) {
        if self.nested() {
            return self.json.number(buf, value, source);
        }
        buf.extend(value.as_bytes());
    }

    #[inline]
    fn boolean(&mut self, buf: &mut Vec<u8>, value: bool) {
        self.json.boolean(buf, value);
    }

    #[inline]
    fn null(&mut self, buf: &mut Vec<u8>) {
        self.json.null(buf);
    }

    #[inline]
    fn begin_object(&mut self, buf: &mut Vec<u8>) {
        self.begin_nested(buf);
        self.json.begin_object(buf);
    }

    #[inline]
    fn end_object(&mut self, buf: &mut Vec<u8>) {
        self.json.end_object(buf);
        self.end_nested(buf);
    }

    #[inline]
    fn begin_array(&mut self, buf: &mut Vec<u8>) {
        self.begin_nested(buf);
        self.json.begin_array(buf);
    }

    #[inline]
    fn end_array(&mut self, buf: &mut Vec<u8>) {
        self.json.end_array(buf);
        self.end_nested(buf);
    }

    #[inline]
    fn plain(&mut self, buf: &mut Vec<u8>, write: impl FnOnce(&mut Vec<u8>) -> bool) -> bool {
        if self.nested() {
            return self.json.plain(buf, write);
        }
        let begin = buf.len();
        if write(buf) {
            true
        } else {
            buf.truncate(begin);
            false
        }
    }
}

// ---

/// Tells whether a string value can be written unquoted, so that it reads back as the same string.
#[inline]
fn is_bare(value: &[u8]) -> bool {
    str::from_utf8(value).is_ok_and(is_bare_str)
}

/// Tells whether a string value can be written unquoted, so that it reads back as the same string.
#[inline]
fn is_bare_str(value: &str) -> bool {
    is_bare_value(value) && reads_as_string(value)
}

/// Tells whether an unquoted value is read back as a string rather than as a number, boolean or null.
#[inline]
fn reads_as_string(value: &str) -> bool {
    matches!(RawValue::auto(value), RawValue::String(_))
}

/// Turns the compact JSON at `buf[begin..]` into a JSON string holding it, in place.
///
/// Compact JSON has no control characters, so only quotes and backslashes need escaping.
/// The bytes are moved from the end, as each of them moves to the same position or a later one.
fn quote_in_place(buf: &mut Vec<u8>, begin: usize) {
    let end = buf.len();
    let escapes = buf[begin..].iter().filter(|&&b| b == b'"' || b == b'\\').count();
    buf.resize(end + escapes + 2, 0);
    *buf.last_mut().unwrap() = b'"';
    // Short runs between escaped bytes, as in objects with short strings, are moved a byte at a time,
    // and longer ones a run at a time.
    if end - begin < QUOTE_RUN_THRESHOLD * (escapes + 1) {
        let mut dst = buf.len() - 1;
        for src in (begin..end).rev() {
            let b = buf[src];
            dst -= 1;
            buf[dst] = b;
            if b == b'"' || b == b'\\' {
                dst -= 1;
                buf[dst] = b'\\';
            }
        }
        debug_assert_eq!(dst, begin + 1);
    } else {
        // How far the bytes before `tail` move, which is one less after each escaped byte.
        let mut shift = escapes + 1;
        let mut tail = end;
        while let Some(i) = memchr::memrchr2(b'"', b'\\', &buf[begin..tail]).map(|i| begin + i) {
            buf.copy_within(i + 1..tail, i + 1 + shift);
            buf[i + shift] = buf[i];
            buf[i + shift - 1] = b'\\';
            shift -= 1;
            tail = i;
        }
        debug_assert_eq!(shift, 1);
        buf.copy_within(begin..tail, begin + 1);
    }
    buf[begin] = b'"';
}

/// The average length of runs between escaped bytes from which they are moved a run at a time.
const QUOTE_RUN_THRESHOLD: usize = 16;

#[cfg(test)]
mod tests;
