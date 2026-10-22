//! JSON encoding of structured output.

// workspace imports
use encstr::{AnyEncodedString, EncodedString};

// relative imports
use super::{Encoder, NumberSource};

// ---

/// Encodes records as compact JSON objects, one per record.
#[derive(Clone, Copy, Default)]
pub struct JsonEncoder {
    /// Whether the next key or array item is the first one in its object or array.
    first: bool,
}

impl JsonEncoder {
    #[inline]
    fn separate(&mut self, buf: &mut Vec<u8>) {
        if !self.first {
            buf.push(b',');
        }
        self.first = false;
    }
}

impl Encoder for JsonEncoder {
    #[inline]
    fn begin_record(&mut self, buf: &mut Vec<u8>) {
        self.begin_object(buf);
    }

    #[inline]
    fn end_record(&mut self, buf: &mut Vec<u8>) {
        self.end_object(buf);
    }

    #[inline]
    fn key(&mut self, buf: &mut Vec<u8>, key: &str) {
        self.separate(buf);
        push_str(buf, key);
        buf.push(b':');
    }

    #[inline]
    fn item(&mut self, buf: &mut Vec<u8>) {
        self.separate(buf);
    }

    #[inline]
    fn string(&mut self, buf: &mut Vec<u8>, value: EncodedString) {
        push_encoded_str(buf, value);
    }

    #[inline]
    fn text(&mut self, buf: &mut Vec<u8>, parts: &[&str]) {
        buf.push(b'"');
        for part in parts {
            push_str_content(buf, part);
        }
        buf.push(b'"');
    }

    #[inline]
    fn number(&mut self, buf: &mut Vec<u8>, value: &str, source: NumberSource) {
        match source {
            NumberSource::Json => buf.extend(value.as_bytes()),
            NumberSource::Logfmt => push_number(buf, value),
        }
    }

    #[inline]
    fn boolean(&mut self, buf: &mut Vec<u8>, value: bool) {
        buf.extend(if value { &b"true"[..] } else { &b"false"[..] });
    }

    #[inline]
    fn null(&mut self, buf: &mut Vec<u8>) {
        buf.extend(b"null");
    }

    #[inline]
    fn begin_object(&mut self, buf: &mut Vec<u8>) {
        buf.push(b'{');
        self.first = true;
    }

    #[inline]
    fn end_object(&mut self, buf: &mut Vec<u8>) {
        buf.push(b'}');
        self.first = false;
    }

    #[inline]
    fn begin_array(&mut self, buf: &mut Vec<u8>) {
        buf.push(b'[');
        self.first = true;
    }

    #[inline]
    fn end_array(&mut self, buf: &mut Vec<u8>) {
        buf.push(b']');
        self.first = false;
    }
}

// ---

/// Writes an encoded string as a JSON string.
///
/// A JSON-encoded string with no escapes and no control characters is copied verbatim.
/// Otherwise it is decoded and encoded again, which normalizes escapes such as `\/`
/// and replaces invalid sequences, such as lone surrogates, with U+FFFD.
#[inline]
pub fn push_encoded_str(buf: &mut Vec<u8>, value: EncodedString) {
    match value {
        EncodedString::Json(value) => {
            let source = value.source();
            if source.bytes().all(|b| b != b'\\' && b >= 0x20) {
                buf.extend(source.as_bytes());
            } else {
                buf.push(b'"');
                decode_json_str(&source[1..source.len() - 1], |s| push_str_content(buf, s));
                buf.push(b'"');
            }
        }
        EncodedString::Raw(value) => push_str(buf, value.as_str()),
    }
}

/// Writes plain text as a JSON string.
#[inline]
pub fn push_str(buf: &mut Vec<u8>, value: &str) {
    buf.push(b'"');
    push_str_content(buf, value);
    buf.push(b'"');
}

/// Writes plain text as the content of a JSON string, escaping where required.
/// Characters other than quotes, backslashes and control characters are written as is.
#[inline]
fn push_str_content(buf: &mut Vec<u8>, value: &str) {
    let mut bytes = value.as_bytes();
    while let Some(pos) = bytes.iter().position(|&b| b < 0x20 || b == b'"' || b == b'\\') {
        buf.extend(&bytes[..pos]);
        push_escape(buf, bytes[pos]);
        bytes = &bytes[pos + 1..];
    }
    buf.extend(bytes);
}

#[inline]
fn push_escape(buf: &mut Vec<u8>, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    buf.push(b'\\');
    match byte {
        b'"' | b'\\' => buf.push(byte),
        b'\n' => buf.push(b'n'),
        b'\r' => buf.push(b'r'),
        b'\t' => buf.push(b't'),
        b'\x08' => buf.push(b'b'),
        b'\x0c' => buf.push(b'f'),
        _ => buf.extend([b'u', b'0', b'0', HEX[(byte >> 4) as usize], HEX[(byte & 0xf) as usize]]),
    }
}

/// Decodes the content of a JSON string, passing the decoded text to `push` in pieces.
///
/// Invalid escapes and lone surrogates are decoded as U+FFFD.
pub fn decode_json_str(mut s: &str, mut push: impl FnMut(&str)) {
    const REPLACEMENT: &str = "\u{fffd}";

    while let Some(pos) = s.find('\\') {
        push(&s[..pos]);
        s = &s[pos + 1..];
        let mut chars = s.chars();
        let decoded = match chars.next() {
            Some('"') => '"',
            Some('\\') => '\\',
            Some('/') => '/',
            Some('b') => '\x08',
            Some('f') => '\x0c',
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some('u') => {
                let (decoded, rest) = decode_unicode_escape(&s[1..]);
                s = rest;
                let mut tmp = [0; 4];
                push(decoded.map(|c| &*c.encode_utf8(&mut tmp)).unwrap_or(REPLACEMENT));
                continue;
            }
            Some(_) | None => {
                push(REPLACEMENT);
                s = chars.as_str();
                continue;
            }
        };
        let mut tmp = [0; 4];
        push(decoded.encode_utf8(&mut tmp));
        s = chars.as_str();
    }
    push(s);
}

/// Decodes the hex digits of a `\u` escape at the start of `s`, including a following
/// low surrogate escape for a high surrogate. Returns the character, or `None` for an
/// invalid escape or a lone surrogate, and the rest of `s`.
fn decode_unicode_escape(s: &str) -> (Option<char>, &str) {
    let hex = |s: &str| {
        s.get(..4)
            .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|h| u16::from_str_radix(h, 16).ok())
    };
    let Some(n1) = hex(s) else {
        return (None, s);
    };
    let s = &s[4..];
    match n1 {
        0xD800..=0xDBFF => match s.strip_prefix("\\u").and_then(|rest| Some((hex(rest)?, &rest[4..]))) {
            Some((n2 @ 0xDC00..=0xDFFF, rest)) => {
                let n = (((n1 - 0xD800) as u32) << 10 | (n2 - 0xDC00) as u32) + 0x1_0000;
                (char::from_u32(n), rest)
            }
            _ => (None, s),
        },
        0xDC00..=0xDFFF => (None, s),
        n => (char::from_u32(n as u32), s),
    }
}

/// Writes a number in valid JSON syntax with the same value.
///
/// The input follows hl's number syntax, which also allows a leading `+`, redundant leading
/// zeros, a missing integer part and a trailing `.` with no fraction digits.
/// Numbers that are already valid JSON are written verbatim.
#[inline]
pub fn push_number(buf: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    let (sign, rest) = match bytes.first() {
        Some(b'-') => (&b"-"[..], &bytes[1..]),
        Some(b'+') => (&b""[..], &bytes[1..]),
        _ => (&b""[..], bytes),
    };
    let int_len = rest.iter().take_while(|b| b.is_ascii_digit()).count();
    let (int, rest) = rest.split_at(int_len);
    let (fraction, exponent) = match rest.first() {
        Some(b'.') => {
            let n = rest[1..].iter().take_while(|b| b.is_ascii_digit()).count();
            (&rest[1..1 + n], &rest[1 + n..])
        }
        _ => (&b""[..], rest),
    };
    let int = match int.iter().position(|&b| b != b'0') {
        Some(pos) => &int[pos..],
        None => &b"0"[..],
    };

    buf.extend(sign);
    buf.extend(int);
    if !fraction.is_empty() {
        buf.push(b'.');
        buf.extend(fraction);
    }
    buf.extend(exponent);
}

#[cfg(test)]
mod tests;
