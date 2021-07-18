// std imports
use std::sync::Arc;

// third-party imports
use chrono::prelude::*;
use json::{de::Read, de::StrRead, value::RawValue};
use serde_json as json;

// local imports
use crate::datefmt;
use crate::filtering::IncludeExcludeSetting;
use crate::fmtx;
use crate::model;
use crate::theme;
use crate::IncludeExcludeKeyFilter;

use datefmt::DateTimeFormatter;
use fmtx::{aligned_left, centered, Counter};
use model::Level;
use theme::{Element, StylingPush, Theme};

// ---

type Buf = Vec<u8>;

// ---

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

    pub fn format_record(&mut self, buf: &mut Buf, rec: &model::Record) {
        self.theme.apply(buf, &rec.level, |s| {
            //
            // time
            //
            s.element(Element::Time, |s| {
                s.batch(|buf| {
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
                })
            });
            //
            // level
            //
            s.batch(|buf| buf.push(b' '));
            s.element(Element::Delimiter, |s| s.batch(|buf| buf.push(b'|')));
            s.element(Element::Level, |s| {
                s.batch(|buf| {
                    buf.extend_from_slice(match rec.level {
                        Some(Level::Debug) => b"DBG",
                        Some(Level::Info) => b"INF",
                        Some(Level::Warning) => b"WRN",
                        Some(Level::Error) => b"ERR",
                        _ => b"(?)",
                    })
                })
            });
            s.element(Element::Delimiter, |s| s.batch(|buf| buf.push(b'|')));
            //
            // logger
            //
            if let Some(logger) = rec.logger {
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Logger, |s| {
                    s.batch(|buf| {
                        buf.extend_from_slice(logger.as_bytes());
                        buf.push(b':');
                    })
                });
            }
            //
            // message text
            //
            if let Some(text) = rec.message {
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Message, |s| {
                    self.format_message(s, text);
                });
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
                    some_fields_hidden |= !self.format_field(s, k, v, Some(&self.fields));
                }
            }
            if some_fields_hidden {
                s.element(Element::Ellipsis, |s| {
                    s.batch(|buf| buf.extend_from_slice(b" ..."))
                });
            }
            //
            // caller
            //
            if let Some(text) = rec.caller {
                // println!("P1");
                s.element(Element::AtSign, |s| {
                    s.batch(|buf| buf.extend_from_slice(b" @ "))
                });
                // println!("P2");
                s.element(Element::Caller, |s| {
                    s.batch(|buf| buf.extend_from_slice(text.as_bytes()))
                });
                // println!("P3");
            };
            //
            // eol
            //
            s.batch(|buf| buf.push(b'\n'));
        });
    }

    fn format_field<S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        key: &str,
        value: &RawValue,
        filter: Option<&IncludeExcludeKeyFilter>,
    ) -> bool {
        let mut fv = FieldFormatter::new(self);
        fv.format(s, key, value, filter, IncludeExcludeSetting::Unspecified)
    }

    fn format_value<S: StylingPush<Buf>>(&self, s: &mut S, value: &RawValue) {
        let mut fv = FieldFormatter::new(self);
        fv.format_value(s, value, None, IncludeExcludeSetting::Unspecified);
    }

    fn format_message<S: StylingPush<Buf>>(&self, s: &mut S, value: &RawValue) {
        match value.get().as_bytes()[0] {
            b'"' => {
                s.element(Element::Message, |s| {
                    s.batch(|buf| format_str_unescaped(buf, value.get()))
                });
            }
            b'0'..=b'9' | b'-' | b'+' | b'.' => {
                s.element(Element::Number, |s| {
                    s.batch(|buf| buf.extend_from_slice(value.get().as_bytes()))
                });
            }
            b't' | b'f' => {
                s.element(Element::Boolean, |s| {
                    s.batch(|buf| buf.extend_from_slice(value.get().as_bytes()))
                });
            }
            b'n' => {
                s.element(Element::Null, |s| {
                    s.batch(|buf| buf.extend_from_slice(value.get().as_bytes()))
                });
            }
            b'{' => {
                let item = json::from_str::<model::Object>(value.get()).unwrap();
                s.element(Element::Brace, |s| s.batch(|buf| buf.push(b'{')));
                let mut has_some = false;
                for (k, v) in item.fields.iter() {
                    has_some |= self.format_field(s, k, v, None)
                }
                if has_some {
                    s.batch(|buf| buf.push(b' '));
                }
                s.element(Element::Brace, |s| s.batch(|buf| buf.push(b'}')));
            }
            b'[' => {
                let item = json::from_str::<model::Array<256>>(value.get()).unwrap();
                let is_byte_string = item
                    .iter()
                    .map(|&v| {
                        let v = v.get().as_bytes();
                        only_digits(v) && (v.len() < 3 || (v.len() == 3 && v <= &b"255"[..]))
                    })
                    .position(|x| x == false)
                    .is_none();
                if is_byte_string {
                    s.element(Element::Quote, |s| {
                        s.batch(|buf| buf.extend_from_slice(b"b'"))
                    });
                    s.element(Element::Message, |s| {
                        for item in item.iter() {
                            let b = atoi::atoi::<u8>(item.get().as_bytes()).unwrap();
                            if b >= 32 {
                                s.batch(|buf| buf.push(b));
                            } else {
                                s.element(Element::String, |s| {
                                    s.batch(|buf| {
                                        buf.push(b'\\');
                                        buf.push(HEXDIGIT[(b >> 4) as usize]);
                                        buf.push(HEXDIGIT[(b & 0xF) as usize]);
                                    })
                                });
                            }
                        }
                    });
                    s.element(Element::Quote, |s| s.batch(|buf| buf.push(b'\'')));
                } else {
                    s.element(Element::Brace, |s| s.batch(|buf| buf.push(b'[')));
                    let mut first = true;
                    for v in item.iter() {
                        if !first {
                            s.element(Element::Comma, |s| s.batch(|buf| buf.push(b',')));
                        } else {
                            first = false;
                        }
                        self.format_value(s, v);
                    }
                    s.element(Element::Brace, |s| s.batch(|buf| buf.push(b']')));
                }
            }
            _ => {
                s.element(Element::Message, |s| {
                    s.batch(|buf| buf.extend_from_slice(value.get().as_bytes()))
                });
            }
        };
    }
}

fn format_str_unescaped(buf: &mut Buf, s: &str) {
    let mut reader = StrRead::new(&s[1..]);
    reader.parse_str_raw(buf).unwrap();
}

struct FieldFormatter<'a> {
    rf: &'a RecordFormatter,
}

impl<'a> FieldFormatter<'a> {
    fn new(rf: &'a RecordFormatter) -> Self {
        Self { rf }
    }

    fn format<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
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
        s.batch(|buf| buf.push(b' '));
        s.element(Element::FieldKey, |s| {
            for b in key.as_bytes() {
                let b = if *b == b'_' { b'-' } else { *b };
                s.batch(|buf| buf.push(b.to_ascii_lowercase()));
            }
        });
        s.element(Element::EqualSign, |s| s.batch(|buf| buf.push(b'=')));
        if self.rf.unescape_fields {
            self.format_value(s, value, filter, setting);
        } else {
            s.element(Element::String, |s| {
                s.batch(|buf| buf.extend_from_slice(value.get().as_bytes()))
            });
        }
        true
    }

    fn format_value<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        value: &'a RawValue,
        filter: Option<&IncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
    ) {
        match value.get().as_bytes()[0] {
            b'"' => {
                s.element(Element::Quote, |s| s.batch(|buf| buf.push(b'\'')));
                s.element(Element::String, |s| {
                    s.batch(|buf| format_str_unescaped(buf, value.get()))
                });
                s.element(Element::Quote, |s| s.batch(|buf| buf.push(b'\'')));
            }
            b'0'..=b'9' | b'-' | b'+' | b'.' => {
                s.element(Element::Number, |s| {
                    s.batch(|buf| buf.extend_from_slice(value.get().as_bytes()))
                });
            }
            b't' | b'f' => {
                s.element(Element::Boolean, |s| {
                    s.batch(|buf| buf.extend_from_slice(value.get().as_bytes()))
                });
            }
            b'n' => {
                s.element(Element::Null, |s| {
                    s.batch(|buf| buf.extend_from_slice(value.get().as_bytes()))
                });
            }
            b'{' => {
                let item = json::from_str::<model::Object>(value.get()).unwrap();
                s.element(Element::Brace, |s| s.batch(|buf| buf.push(b'{')));
                let mut some_fields_hidden = false;
                for (k, v) in item.fields.iter() {
                    some_fields_hidden |= !self.format(s, k, v, filter, setting);
                }
                if some_fields_hidden {
                    s.element(Element::Ellipsis, |s| {
                        s.batch(|buf| buf.extend_from_slice(b" ..."))
                    });
                }
                if item.fields.len() != 0 {
                    s.batch(|buf| buf.push(b' '));
                }
                s.element(Element::Brace, |s| s.batch(|buf| buf.push(b'}')));
            }
            b'[' => {
                let item = json::from_str::<model::Array<32>>(value.get()).unwrap();
                s.element(Element::Brace, |s| s.batch(|buf| buf.push(b'[')));
                let mut first = true;
                for v in item.iter() {
                    if !first {
                        s.element(Element::Comma, |s| s.batch(|buf| buf.push(b',')));
                    } else {
                        first = false;
                    }
                    self.format_value(s, v, None, IncludeExcludeSetting::Unspecified);
                }
                s.element(Element::Brace, |s| s.batch(|buf| buf.push(b']')));
            }
            _ => {
                s.element(Element::String, |s| {
                    s.batch(|buf| buf.extend_from_slice(value.get().as_bytes()))
                });
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
