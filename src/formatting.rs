use std::sync::Arc;

use chrono::prelude::*;
use json::{de::Read, de::StrRead, value::RawValue};
use serde_json as json;

use crate::datefmt;
use crate::filtering::IncludeExcludeSetting;
use crate::fmtx;
use crate::model;
use crate::theme;
use crate::IncludeExcludeKeyFilter;

use datefmt::DateTimeFormatter;
use fmtx::{aligned_left, centered, Counter};
use model::Level;
use theme::{Element, Styler, Theme};

pub struct RecordFormatter {
    theme: Arc<Theme>,
    unescape_fields: bool,
    ts_formatter: DateTimeFormatter,
    ts_width: usize,
    hide_empty_fields: bool,
    fields: Arc<IncludeExcludeKeyFilter>,
}

impl RecordFormatter {
    pub fn new(
        theme: Arc<Theme>,
        ts_formatter: DateTimeFormatter,
        hide_empty_fields: bool,
        fields: Arc<IncludeExcludeKeyFilter>,
    ) -> Self {
        let mut counter = Counter::new();
        let tts = Utc.ymd(2020, 12, 30).and_hms_nano(23, 59, 49, 999_999_999);
        ts_formatter.format(&mut counter, tts.into());
        let ts_width = counter.result();
        RecordFormatter {
            theme,
            unescape_fields: true,
            ts_formatter,
            ts_width,
            hide_empty_fields,
            fields,
        }
    }

    pub fn with_field_unescaping(mut self, value: bool) -> Self {
        self.unescape_fields = value;
        self
    }

    pub fn format_record(&mut self, buf: &mut Vec<u8>, rec: &model::Record) {
        self.theme.apply(buf, &rec.level, |buf, styler| {
            //
            // time
            //
            styler.set(buf, Element::Time);
            if let Some(ts) = &rec.ts {
                aligned_left(buf, self.ts_width, b' ', |mut buf| {
                    if ts
                        .as_rfc3339()
                        .and_then(|ts| self.ts_formatter.reformat_rfc3339(&mut buf, ts))
                        .is_none()
                    {
                        if let Some(ts) = ts.parse() {
                            self.ts_formatter.format(&mut buf, ts);
                        } else {
                            buf.extend_from_slice(ts.raw().as_bytes());
                        }
                    }
                });
            } else {
                centered(buf, self.ts_width, b' ', |mut buf| {
                    buf.extend_from_slice(b"---");
                });
            }
            //
            // level
            //
            styler.set(buf, Element::Whitespace);
            buf.push(b' ');
            styler.set(buf, Element::Delimiter);
            buf.push(b'|');
            styler.set(buf, Element::Level);
            buf.extend_from_slice(match rec.level {
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
            if let Some(logger) = rec.logger {
                styler.set(buf, Element::Whitespace);
                buf.push(b' ');
                styler.set(buf, Element::Logger);
                buf.extend_from_slice(logger.as_bytes());
                buf.push(b':');
            }
            //
            // message text
            //
            if let Some(text) = rec.message {
                styler.set(buf, Element::Whitespace);
                buf.push(b' ');
                self.format_message(buf, styler, text);
            }
            //
            // fields
            //
            let mut some_fields_hidden = false;
            for (k, v) in rec.fields() {
                if !self.hide_empty_fields
                    || match v.get() {
                        r#""""# | "null" | "{}" | "[]" => false,
                        _ => true,
                    }
                {
                    some_fields_hidden |= !self.format_field(buf, styler, k, v, Some(&self.fields));
                }
            }
            if some_fields_hidden {
                styler.set(buf, Element::Ellipsis);
                buf.extend_from_slice(b" ...");
            }
            //
            // caller
            //
            if let Some(text) = rec.caller {
                styler.set(buf, Element::AtSign);
                buf.extend_from_slice(b" @ ");
                styler.set(buf, Element::Caller);
                buf.extend_from_slice(text.as_bytes());
            };
        });
        //
        // eol
        //
        buf.push(b'\n');
    }

    fn format_field<'a, 'b: 'a>(
        &self,
        buf: &'a mut Vec<u8>,
        styler: &'a mut Styler<'b>,
        key: &str,
        value: &RawValue,
        filter: Option<&IncludeExcludeKeyFilter>,
    ) -> bool {
        let mut fv = FieldFormatter::new(self, buf, styler);
        fv.format(key, value, filter, IncludeExcludeSetting::Unspecified)
    }

    fn format_value<'a, 'b: 'a>(
        &self,
        buf: &'a mut Vec<u8>,
        styler: &'a mut Styler<'b>,
        value: &RawValue,
    ) {
        let mut fv = FieldFormatter::new(self, buf, styler);
        fv.format_value(value, None, IncludeExcludeSetting::Unspecified);
    }

    fn format_message<'a, 'b: 'a>(
        &self,
        buf: &'a mut Vec<u8>,
        styler: &'a mut Styler<'b>,
        value: &RawValue,
    ) {
        match value.get().as_bytes()[0] {
            b'"' => {
                styler.set(buf, Element::Message);
                format_str_unescaped(buf, value.get());
            }
            b'0'..=b'9' | b'-' | b'+' | b'.' => {
                styler.set(buf, Element::Number);
                buf.extend_from_slice(value.get().as_bytes());
            }
            b't' | b'f' => {
                styler.set(buf, Element::Boolean);
                buf.extend_from_slice(value.get().as_bytes());
            }
            b'n' => {
                styler.set(buf, Element::Null);
                buf.extend_from_slice(value.get().as_bytes());
            }
            b'{' => {
                let item = json::from_str::<model::Object>(value.get()).unwrap();
                styler.set(buf, Element::Brace);
                buf.push(b'{');
                let mut has_some = false;
                for (k, v) in item.fields.iter() {
                    has_some |= self.format_field(buf, styler, k, v, None)
                }
                styler.set(buf, Element::Brace);
                if has_some {
                    buf.push(b' ');
                }
                buf.push(b'}');
            }
            b'[' => {
                let item = json::from_str::<model::Array<256>>(value.get()).unwrap();
                let is_byte_string = item
                    .iter()
                    .map(|&v| {
                        let v = v.get().as_bytes();
                        only_digits(v) && (v.len() < 3 || (v.len() == 3 && v <= b"255"))
                    })
                    .position(|x| x == false)
                    .is_none();
                if is_byte_string {
                    styler.set(buf, Element::Quote);
                    buf.push(b'b');
                    buf.push(b'\'');
                    for item in item.iter() {
                        let b = atoi::atoi::<u8>(item.get().as_bytes()).unwrap();
                        if b >= 32 {
                            styler.set(buf, Element::Message);
                            buf.push(b);
                        } else {
                            styler.set(buf, Element::String);
                            buf.push(b'\\');
                            buf.push(HEXDIGIT[(b >> 4) as usize]);
                            buf.push(HEXDIGIT[(b & 0xF) as usize]);
                        }
                    }
                    styler.set(buf, Element::Quote);
                    buf.push(b'\'');
                } else {
                    styler.set(buf, Element::Brace);
                    buf.push(b'[');
                    let mut first = true;
                    for v in item.iter() {
                        if !first {
                            styler.set(buf, Element::Comma);
                            buf.push(b',');
                        } else {
                            first = false;
                        }
                        self.format_value(buf, styler, v);
                    }
                    styler.set(buf, Element::Brace);
                    buf.push(b']');
                }
            }
            _ => {
                styler.set(buf, Element::Message);
                buf.extend_from_slice(value.get().as_bytes());
            }
        };
    }
}

fn format_str_unescaped(buf: &mut Vec<u8>, s: &str) {
    let mut reader = StrRead::new(&s[1..]);
    reader.parse_str_raw(buf).unwrap();
}

struct FieldFormatter<'a, 'b> {
    rf: &'a RecordFormatter,
    buf: &'a mut Vec<u8>,
    styler: &'a mut Styler<'b>,
}

impl<'a, 'b> FieldFormatter<'a, 'b> {
    fn new(rf: &'a RecordFormatter, buf: &'a mut Vec<u8>, styler: &'a mut Styler<'b>) -> Self {
        Self { rf, buf, styler }
    }

    fn format(
        &mut self,
        key: &str,
        value: &'a RawValue,
        filter: Option<&IncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
    ) -> bool {
        let (filter, setting, leaf) = match filter {
            Some(filter) => {
                let setting = setting.apply(filter.setting());
                match filter.get(key) {
                    Some(filter) => (Some(filter), setting.apply(filter.setting()), filter.leaf()),
                    None => (None, setting, true),
                }
            }
            None => (None, setting, true),
        };
        if setting == IncludeExcludeSetting::Exclude && leaf {
            return false;
        }
        self.buf.push(b' ');
        self.styler.set(self.buf, Element::FieldKey);
        for b in key.as_bytes() {
            let b = if *b == b'_' { b'-' } else { *b };
            self.buf.push(b.to_ascii_lowercase());
        }
        self.styler.set(self.buf, Element::EqualSign);
        self.buf.push(b'=');
        if self.rf.unescape_fields {
            self.format_value(value, filter, setting);
        } else {
            self.styler.set(self.buf, Element::String);
            self.buf.extend_from_slice(value.get().as_bytes())
        }
        true
    }

    fn format_value(
        &mut self,
        value: &'a RawValue,
        filter: Option<&IncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
    ) {
        match value.get().as_bytes()[0] {
            b'"' => {
                self.styler.set(self.buf, Element::Quote);
                self.buf.push(b'\'');
                self.styler.set(self.buf, Element::String);
                format_str_unescaped(self.buf, value.get());
                self.styler.set(self.buf, Element::Quote);
                self.buf.push(b'\'');
            }
            b'0'..=b'9' | b'-' | b'+' | b'.' => {
                self.styler.set(self.buf, Element::Number);
                self.buf.extend_from_slice(value.get().as_bytes());
            }
            b't' | b'f' => {
                self.styler.set(self.buf, Element::Boolean);
                self.buf.extend_from_slice(value.get().as_bytes());
            }
            b'n' => {
                self.styler.set(self.buf, Element::Null);
                self.buf.extend_from_slice(value.get().as_bytes());
            }
            b'{' => {
                let item = json::from_str::<model::Object>(value.get()).unwrap();
                self.styler.set(self.buf, Element::Brace);
                self.buf.push(b'{');
                let mut some_fields_hidden = false;
                for (k, v) in item.fields.iter() {
                    some_fields_hidden |= !self.format(k, v, filter, setting);
                }
                if some_fields_hidden {
                    self.styler.set(self.buf, Element::Ellipsis);
                    self.buf.extend_from_slice(b" ...");
                }
                if item.fields.len() != 0 {
                    self.buf.push(b' ');
                }
                self.styler.set(self.buf, Element::Brace);
                self.buf.push(b'}');
            }
            b'[' => {
                let item = json::from_str::<model::Array<32>>(value.get()).unwrap();
                self.styler.set(self.buf, Element::Brace);
                self.buf.push(b'[');
                let mut first = true;
                for v in item.iter() {
                    if !first {
                        self.styler.set(self.buf, Element::Comma);
                        self.buf.push(b',');
                    } else {
                        first = false;
                    }
                    self.format_value(v, None, IncludeExcludeSetting::Unspecified);
                }
                self.styler.set(self.buf, Element::Brace);
                self.buf.push(b']');
            }
            _ => {
                self.styler.set(self.buf, Element::String);
                self.buf.extend_from_slice(value.get().as_bytes());
            }
        };
    }
}

fn only_digits(b: &[u8]) -> bool {
    b.iter().position(|&b| !b.is_ascii_digit()).is_none()
}

const HEXDIGIT: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];
