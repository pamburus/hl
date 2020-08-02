use std::sync::Arc;

use chrono::naive::NaiveDateTime;
use chrono::{DateTime, Datelike, Timelike};
use json::{de::Read, de::StrRead, value::RawValue};
use serde_json as json;

use crate::model;
use crate::theme;

use model::Level;
use theme::{Element, Styler, Theme};

pub struct MessageFormatter {
    theme: Arc<Theme>,
    unescape_fields: bool,
}

impl MessageFormatter {
    pub fn new(theme: Arc<Theme>) -> Self {
        Self {
            theme,
            unescape_fields: true,
        }
    }

    pub fn with_field_unescaping(mut self, value: bool) -> Self {
        self.unescape_fields = value;
        self
    }

    pub fn format_message(&mut self, buf: &mut Vec<u8>, msg: &model::Message) {
        self.theme.apply(buf, &msg.level, |buf, styler| {
            //
            // time
            //
            styler.set(buf, Element::Time);
            if let Some(ts) = msg.ts {
                let mut format = || -> Option<()> {
                    let s = json::from_str::<&str>(ts.get()).ok()?;
                    let bytes = s.as_bytes();
                    if only_digits(bytes) {
                        let (ts, nsec) = parse_unix_timestamp(bytes)?;
                        let ts = NaiveDateTime::from_timestamp_opt(ts, nsec)?;
                        let ts = DateTime::from_utc(ts, chrono::Utc);
                        format_date(buf, ts);
                        Some(())
                    } else if is_rfc_3339(bytes) {
                        let (date, time) = split_rfc3339(bytes);
                        buf.extend_from_slice(date);
                        buf.push(b' ');
                        buf.extend_from_slice(time);
                        Some(())
                    } else {
                        None
                    }
                };
                if format().is_none() {
                    buf.extend_from_slice(b"    <<< bad time >>>   ");
                }
            } else {
                buf.extend_from_slice(b"  <<< missing time >>> ");
            }
            //
            // level
            //
            buf.push(b' ');
            styler.set(buf, Element::Delimiter);
            buf.push(b'|');
            styler.set(buf, Element::Level);
            buf.extend_from_slice(match msg.level {
                Some(Level::Debug) => b"DBG",
                Some(Level::Info) => b"INF",
                Some(Level::Warning) => b"WRN",
                Some(Level::Error) => b"ERR",
                _ => b"(?)",
            });
            styler.set(buf, Element::Delimiter);
            buf.push(b'|');
            //
            // logger
            //
            if let Some(logger) = msg.logger {
                buf.push(b' ');
                styler.set(buf, Element::Logger);
                buf.extend_from_slice(logger.as_bytes());
                styler.set(buf, Element::Punctuation);
                buf.push(b':');
            }
            //
            // message
            //
            if let Some(text) = msg.text {
                buf.push(b' ');
                styler.set(buf, Element::Message);
                format_str_unescaped(buf, text.get());
            }
            //
            // fields
            //
            for (k, v) in msg.fields() {
                buf.push(b' ');
                self.format_field_value(buf, styler, k, v);
            }
            //
            // caller
            //
            if let Some(text) = msg.caller {
                styler.set(buf, Element::Punctuation);
                buf.extend_from_slice(b" @ ");
                styler.set(buf, Element::Caller);
                buf.extend_from_slice(text.as_bytes());
            };
            //
            // eol
            //
            buf.push(b'\n');
        });
    }

    fn format_field_value<'a, 'b: 'a>(
        &self,
        buf: &'a mut Vec<u8>,
        styler: &'a mut Styler<'b>,
        key: &str,
        value: &RawValue,
    ) {
        let mut fv = FieldFormatter::new(self, buf, styler);
        fv.format(key, value);
    }
}

fn format_str_unescaped(buf: &mut Vec<u8>, s: &str) {
    let mut reader = StrRead::new(&s[1..]);
    reader.parse_str_raw(buf).unwrap();
}

struct FieldFormatter<'a, 'b> {
    mf: &'a MessageFormatter,
    buf: &'a mut Vec<u8>,
    styler: &'a mut Styler<'b>,
}

impl<'a, 'b> FieldFormatter<'a, 'b> {
    fn new(mf: &'a MessageFormatter, buf: &'a mut Vec<u8>, styler: &'a mut Styler<'b>) -> Self {
        Self { mf, buf, styler }
    }

    fn format(&mut self, key: &str, value: &'a RawValue) {
        self.styler.set(self.buf, Element::FieldKey);
        for b in key.as_bytes() {
            let b = if *b == b'_' { b'-' } else { *b };
            self.buf.push(b.to_ascii_lowercase());
        }
        self.styler.set(self.buf, Element::EqualSign);
        self.buf.push(b'=');
        if self.mf.unescape_fields {
            self.format_value(value);
        } else {
            self.buf.extend_from_slice(value.get().as_bytes())
        }
    }

    fn format_value(&mut self, value: &'a RawValue) {
        match value.get().as_bytes()[0] {
            b'"' => {
                self.styler.set(self.buf, Element::Quote);
                self.buf.push(b'\'');
                self.styler.set(self.buf, Element::LiteralString);
                format_str_unescaped(self.buf, value.get());
                self.styler.set(self.buf, Element::Quote);
                self.buf.push(b'\'');
            }
            b'0'..=b'9' => {
                self.styler.set(self.buf, Element::LiteralNumber);
                self.buf.extend_from_slice(value.get().as_bytes());
            }
            b't' | b'f' => {
                self.styler.set(self.buf, Element::LiteralBoolean);
                self.buf.extend_from_slice(value.get().as_bytes());
            }
            b'n' => {
                self.styler.set(self.buf, Element::LiteralNull);
                self.buf.extend_from_slice(value.get().as_bytes());
            }
            b'{' => {
                let item = json::from_str::<model::Object>(value.get()).unwrap();
                self.styler.set(self.buf, Element::Brace);
                self.buf.push(b'{');
                for (k, v) in item.fields.iter() {
                    self.buf.push(b' ');
                    self.format(k, v);
                }
                self.styler.set(self.buf, Element::Brace);
                self.buf.extend_from_slice(b" }");
            }
            b'[' => {
                let item = json::from_str::<model::Array>(value.get()).unwrap();
                self.styler.set(self.buf, Element::Brace);
                self.buf.push(b'[');
                let mut first = true;
                for v in item.items.iter() {
                    if !first {
                        self.styler.set(self.buf, Element::Punctuation);
                        self.buf.push(b',');
                    } else {
                        first = false;
                    }
                    self.format_value(v);
                }
                self.styler.set(self.buf, Element::Brace);
                self.buf.push(b']');
            }
            _ => {
                self.styler.set(self.buf, Element::LiteralString);
                self.buf.extend_from_slice(value.get().as_bytes());
            }
        };
    }
}

fn format_date(buf: &mut Vec<u8>, dt: DateTime<chrono::Utc>) {
    format_u32_p2(buf, dt.year() as u32);
    buf.push(b'-');
    format_u32_p2(buf, dt.month());
    buf.push(b'-');
    format_u32_p2(buf, dt.day());
    buf.push(b' ');
    format_u32_p2(buf, dt.hour());
    buf.push(b':');
    format_u32_p2(buf, dt.minute());
    buf.push(b':');
    format_u32_p2(buf, dt.second());
    buf.push(b'.');
    format_u32_p3(buf, dt.nanosecond() / 1000000);
}

fn format_u32_p2(buf: &mut Vec<u8>, v: u32) {
    let d = DIGITS[(v % 100) as usize];
    buf.push(d[0]);
    buf.push(d[1]);
}

fn format_u32_p3(buf: &mut Vec<u8>, v: u32) {
    let d0 = DIGITS[(v % 100) as usize];
    let d1 = DIGITS[(v / 100 % 100) as usize];
    buf.push(d1[1]);
    buf.push(d0[0]);
    buf.push(d0[1]);
}

fn is_rfc_3339(v: &[u8]) -> bool {
    if v.len() < 19 {
        return false;
    }
    if !v[0].is_ascii_digit() {
        return false;
    }
    if !v[1].is_ascii_digit() {
        return false;
    }
    if !v[2].is_ascii_digit() {
        return false;
    }
    if !v[3].is_ascii_digit() {
        return false;
    }
    if v[4] != b'-' {
        return false;
    }
    if !v[5].is_ascii_digit() {
        return false;
    }
    if !v[6].is_ascii_digit() {
        return false;
    }
    if v[7] != b'-' {
        return false;
    }
    if !v[8].is_ascii_digit() {
        return false;
    }
    if !v[9].is_ascii_digit() {
        return false;
    }
    if v[10] != b'T' {
        return false;
    }
    if !v[11].is_ascii_digit() {
        return false;
    }
    if !v[12].is_ascii_digit() {
        return false;
    }
    if v[13] != b':' {
        return false;
    }
    if !v[14].is_ascii_digit() {
        return false;
    }
    if !v[15].is_ascii_digit() {
        return false;
    }
    if v[16] != b':' {
        return false;
    }
    if !v[17].is_ascii_digit() {
        return false;
    }
    if !v[18].is_ascii_digit() {
        return false;
    }

    true
}

fn split_rfc3339<'a>(mut v: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    if v.len() > 23 {
        v = &v[..23]
    }
    for i in 19..23 {
        if v[i] != b'.' && !v[i].is_ascii_digit() {
            v = &v[..i];
            break;
        }
    }
    (&v[2..10], &v[11..])
}

fn parse_unix_timestamp<I: atoi::FromRadix10>(mut text: &[u8]) -> Option<(I, u32)> {
    let n = text.len();
    let nsec = if n > 14 {
        let (nsec, _): (u32, _) = atoi::FromRadix10::from_radix_10(&text[n - 6..]);
        text = &text[..n - 6];
        nsec * 1000
    } else if n > 11 {
        let (nsec, _): (u32, _) = atoi::FromRadix10::from_radix_10(&text[n - 3..]);
        text = &text[..n - 3];
        nsec * 1000000
    } else {
        0
    };

    match I::from_radix_10(text) {
        (_, 0) => None,
        (n, _) => Some((n, nsec)),
    }
}

fn only_digits(b: &[u8]) -> bool {
    b.iter()
        .map(|&b| b.is_ascii_digit())
        .position(|x| x == false)
        .is_none()
}

const DIGITS: [[u8; 2]; 100] = [
    [b'0', b'0'],
    [b'0', b'1'],
    [b'0', b'2'],
    [b'0', b'3'],
    [b'0', b'4'],
    [b'0', b'5'],
    [b'0', b'6'],
    [b'0', b'7'],
    [b'0', b'8'],
    [b'0', b'9'],
    [b'1', b'0'],
    [b'1', b'1'],
    [b'1', b'2'],
    [b'1', b'3'],
    [b'1', b'4'],
    [b'1', b'5'],
    [b'1', b'6'],
    [b'1', b'7'],
    [b'1', b'8'],
    [b'1', b'9'],
    [b'2', b'0'],
    [b'2', b'1'],
    [b'2', b'2'],
    [b'2', b'3'],
    [b'2', b'4'],
    [b'2', b'5'],
    [b'2', b'6'],
    [b'2', b'7'],
    [b'2', b'8'],
    [b'2', b'9'],
    [b'3', b'0'],
    [b'3', b'1'],
    [b'3', b'2'],
    [b'3', b'3'],
    [b'3', b'4'],
    [b'3', b'5'],
    [b'3', b'6'],
    [b'3', b'7'],
    [b'3', b'8'],
    [b'3', b'9'],
    [b'4', b'0'],
    [b'4', b'1'],
    [b'4', b'2'],
    [b'4', b'3'],
    [b'4', b'4'],
    [b'4', b'5'],
    [b'4', b'6'],
    [b'4', b'7'],
    [b'4', b'8'],
    [b'4', b'9'],
    [b'5', b'0'],
    [b'5', b'1'],
    [b'5', b'2'],
    [b'5', b'3'],
    [b'5', b'4'],
    [b'5', b'5'],
    [b'5', b'6'],
    [b'5', b'7'],
    [b'5', b'8'],
    [b'5', b'9'],
    [b'6', b'0'],
    [b'6', b'1'],
    [b'6', b'2'],
    [b'6', b'3'],
    [b'6', b'4'],
    [b'6', b'5'],
    [b'6', b'6'],
    [b'6', b'7'],
    [b'6', b'8'],
    [b'6', b'9'],
    [b'7', b'0'],
    [b'7', b'1'],
    [b'7', b'2'],
    [b'7', b'3'],
    [b'7', b'4'],
    [b'7', b'5'],
    [b'7', b'6'],
    [b'7', b'7'],
    [b'7', b'8'],
    [b'7', b'9'],
    [b'8', b'0'],
    [b'8', b'1'],
    [b'8', b'2'],
    [b'8', b'3'],
    [b'8', b'4'],
    [b'8', b'5'],
    [b'8', b'6'],
    [b'8', b'7'],
    [b'8', b'8'],
    [b'8', b'9'],
    [b'9', b'0'],
    [b'9', b'1'],
    [b'9', b'2'],
    [b'9', b'3'],
    [b'9', b'4'],
    [b'9', b'5'],
    [b'9', b'6'],
    [b'9', b'7'],
    [b'9', b'8'],
    [b'9', b'9'],
];
