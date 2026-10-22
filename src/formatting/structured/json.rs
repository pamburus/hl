//! JSON encoding of structured output.

// workspace imports
use encstr::{AnyEncodedString, EncodedString, Handler, JsonAppender, Token};

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
                // Strings in parsed records are terminated, so decoding can't fail half way.
                _ = value.decode_lossy(JsonAppender::new(buf));
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
#[inline]
fn push_str_content(buf: &mut Vec<u8>, value: &str) {
    JsonAppender::new(buf).handle(Token::Sequence(value));
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
