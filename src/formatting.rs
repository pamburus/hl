// std imports
use std::sync::Arc;

// local imports
use crate::{
    datefmt::DateTimeFormatter,
    filtering::IncludeExcludeSetting,
    fmtx::{aligned_left, centered},
    model::{self, Caller, Level, RawValue, ValueKind},
    settings::Formatting,
    theme::{Element, StylingPush, Theme},
    IncludeExcludeKeyFilter,
};

// ---

type Buf = Vec<u8>;

// ---

pub trait RecordWithSourceFormatter {
    fn format_record(&self, buf: &mut Buf, rec: model::RecordWithSource);
}

pub struct RawRecordFormatter {}

impl RecordWithSourceFormatter for RawRecordFormatter {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, rec: model::RecordWithSource) {
        buf.extend_from_slice(rec.source);
    }
}

impl<T: RecordWithSourceFormatter> RecordWithSourceFormatter for &T {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, rec: model::RecordWithSource) {
        (**self).format_record(buf, rec)
    }
}

impl RecordWithSourceFormatter for Box<dyn RecordWithSourceFormatter> {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, rec: model::RecordWithSource) {
        (**self).format_record(buf, rec)
    }
}

// ---

pub struct RecordFormatter {
    theme: Arc<Theme>,
    unescape_fields: bool,
    ts_formatter: DateTimeFormatter,
    ts_width: usize,
    hide_empty_fields: bool,
    fields: Arc<IncludeExcludeKeyFilter>,
    cfg: Formatting,
}

impl RecordFormatter {
    pub fn new(
        theme: Arc<Theme>,
        ts_formatter: DateTimeFormatter,
        hide_empty_fields: bool,
        fields: Arc<IncludeExcludeKeyFilter>,
        cfg: Formatting,
    ) -> Self {
        let ts_width = ts_formatter.max_length();
        RecordFormatter {
            theme,
            unescape_fields: true,
            ts_formatter,
            ts_width,
            hide_empty_fields,
            fields,
            cfg,
        }
    }

    pub fn with_field_unescaping(mut self, value: bool) -> Self {
        self.unescape_fields = value;
        self
    }

    pub fn format_record(&self, buf: &mut Buf, rec: &model::Record) {
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
            s.space();
            s.element(Element::Level, |s| {
                s.batch(|buf| {
                    buf.extend_from_slice(self.cfg.punctuation.level_left_separator.as_bytes());
                });
                s.element(Element::LevelInner, |s| {
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
                s.batch(|buf| buf.extend_from_slice(self.cfg.punctuation.level_right_separator.as_bytes()));
            });
            //
            // logger
            //
            if let Some(logger) = rec.logger {
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Logger, |s| {
                    s.element(Element::LoggerInner, |s| {
                        s.batch(|buf| buf.extend_from_slice(logger.as_bytes()))
                    });
                    s.batch(|buf| buf.extend_from_slice(self.cfg.punctuation.logger_name_separator.as_bytes()));
                });
            }
            //
            // message text
            //
            if let Some(text) = &rec.message {
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Message, |s| self.format_message(s, *text));
            } else {
                s.reset();
            }
            //
            // fields
            //
            let mut some_fields_hidden = false;
            for (k, v) in rec.fields() {
                if !self.hide_empty_fields || v.is_empty() {
                    some_fields_hidden |= !self.format_field(s, k, *v, Some(&self.fields));
                }
            }
            if some_fields_hidden {
                s.element(Element::Ellipsis, |s| {
                    s.batch(|buf| buf.extend_from_slice(self.cfg.punctuation.hidden_fields_indicator.as_bytes()))
                });
            }
            //
            // caller
            //
            if let Some(caller) = &rec.caller {
                s.element(Element::Caller, |s| {
                    s.batch(|buf| {
                        buf.push(b' ');
                        buf.extend_from_slice(self.cfg.punctuation.source_location_separator.as_bytes())
                    });
                    s.element(Element::CallerInner, |s| {
                        s.batch(|buf| {
                            match caller {
                                Caller::Text(text) => {
                                    buf.extend_from_slice(text.as_bytes());
                                }
                                Caller::FileLine(file, line) => {
                                    buf.extend_from_slice(file.as_bytes());
                                    if line.len() != 0 {
                                        buf.push(b':');
                                        buf.extend_from_slice(line.as_bytes());
                                    }
                                }
                            };
                        });
                    });
                });
            };
        });
    }

    fn format_field<'a, S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        key: &str,
        value: RawValue<'a>,
        filter: Option<&IncludeExcludeKeyFilter>,
    ) -> bool {
        let mut fv = FieldFormatter::new(self);
        fv.format(s, key, value, filter, IncludeExcludeSetting::Unspecified)
    }

    fn format_value<'a, S: StylingPush<Buf>>(&self, s: &mut S, value: RawValue<'a>) {
        let mut fv = FieldFormatter::new(self);
        fv.format_value(s, value, None, IncludeExcludeSetting::Unspecified);
    }

    fn format_message<'a, S: StylingPush<Buf>>(&self, s: &mut S, value: RawValue<'a>) {
        match value.kind() {
            ValueKind::String(_) => {
                s.element(Element::Message, |s| s.batch(|buf| value.format_as_str(buf)));
            }
            ValueKind::Number => {
                s.element(Element::Number, |s| s.batch(|buf| value.format_readable(buf)));
            }
            ValueKind::Boolean => {
                s.element(Element::Boolean, |s| s.batch(|buf| value.format_readable(buf)));
            }
            ValueKind::Null => {
                s.element(Element::Null, |s| s.batch(|buf| value.format_readable(buf)));
            }
            ValueKind::Object => {
                s.element(Element::Object, |s| {
                    let item = value.parse_object().unwrap();
                    s.batch(|buf| buf.push(b'{'));
                    let mut has_some = false;
                    for (k, v) in item.fields.iter() {
                        has_some |= self.format_field(s, k, *v, None)
                    }
                    s.batch(|buf| {
                        if has_some {
                            buf.push(b' ');
                        }
                        buf.push(b'}');
                    });
                });
            }
            ValueKind::Array => {
                let item = value.parse_array::<256>().unwrap();
                let is_byte_string = item
                    .iter()
                    .map(|&v| v.is_byte_code())
                    .position(|x| x == false)
                    .is_none();
                if is_byte_string {
                    s.batch(|buf| buf.extend_from_slice(b"b'"));
                    s.element(Element::Message, |s| {
                        for item in item.iter() {
                            let b = item.parse_byte_code();
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
                    s.batch(|buf| buf.push(b'\''));
                } else {
                    s.element(Element::Array, |s| {
                        s.batch(|buf| buf.push(b'['));
                        let mut first = true;
                        for v in item.iter() {
                            if !first {
                                s.batch(|buf| buf.push(b','));
                            } else {
                                first = false;
                            }
                            self.format_value(s, *v);
                        }
                        s.batch(|buf| buf.push(b']'))
                    });
                }
            }
        };
    }
}

impl RecordWithSourceFormatter for RecordFormatter {
    fn format_record(&self, buf: &mut Buf, rec: model::RecordWithSource) {
        RecordFormatter::format_record(self, buf, rec.record)
    }
}

// ---

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
        value: RawValue<'a>,
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
        s.space();
        s.element(Element::Key, |s| {
            for b in key.as_bytes() {
                let b = if *b == b'_' { b'-' } else { *b };
                s.batch(|buf| buf.push(b));
            }
        });
        s.element(Element::Field, |s| {
            s.batch(|buf| buf.extend_from_slice(self.rf.cfg.punctuation.field_key_value_separator.as_bytes()));
        });
        if self.rf.unescape_fields {
            self.format_value(s, value, filter, setting);
        } else {
            s.element(Element::String, |s| s.batch(|buf| value.format_as_json_str(buf)));
        }
        true
    }

    fn format_value<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        value: RawValue<'a>,
        filter: Option<&IncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
    ) {
        match value.kind() {
            ValueKind::String(_) => {
                s.element(Element::String, |s| {
                    s.batch(|buf| {
                        value.format_as_str(buf);
                    })
                });
            }
            ValueKind::Number => {
                s.element(Element::Number, |s| s.batch(|buf| value.format_readable(buf)));
            }
            ValueKind::Boolean => {
                s.element(Element::Boolean, |s| s.batch(|buf| value.format_readable(buf)));
            }
            ValueKind::Null => {
                s.element(Element::Null, |s| s.batch(|buf| value.format_readable(buf)));
            }
            ValueKind::Object => {
                let item = value.parse_object().unwrap();
                s.element(Element::Object, |s| {
                    s.batch(|buf| buf.push(b'{'));
                    let mut some_fields_hidden = false;
                    for (k, v) in item.fields.iter() {
                        some_fields_hidden |= !self.format(s, k, *v, filter, setting);
                    }
                    if some_fields_hidden {
                        s.element(Element::Ellipsis, |s| s.batch(|buf| buf.extend_from_slice(b" ...")));
                    }
                    s.batch(|buf| {
                        if item.fields.len() != 0 {
                            buf.push(b' ');
                        }
                        buf.push(b'}');
                    });
                });
            }
            ValueKind::Array => {
                s.element(Element::Array, |s| {
                    let item = value.parse_array::<32>().unwrap();
                    s.batch(|buf| buf.push(b'['));
                    let mut first = true;
                    for v in item.iter() {
                        if !first {
                            s.batch(|buf| buf.push(b','));
                        } else {
                            first = false;
                        }
                        self.format_value(s, *v, None, IncludeExcludeSetting::Unspecified);
                    }
                    s.batch(|buf| buf.push(b']'));
                });
            }
        };
    }
}

const HEXDIGIT: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        datefmt::LinuxDateFormat,
        error::Error,
        model::{Record, RecordFields},
        settings::Punctuation,
        theme::Theme,
        themecfg::testing,
        timestamp::Timestamp,
        timezone::Tz,
    };
    use chrono::{Offset, Utc};
    use serde_json as json;

    fn format(rec: &Record) -> Result<String, Error> {
        let formatter = RecordFormatter::new(
            Arc::new(Theme::from(testing::theme()?)),
            DateTimeFormatter::new(
                LinuxDateFormat::new("%y-%m-%d %T.%3N").compile(),
                Tz::FixedOffset(Utc.fix()),
            ),
            false,
            Arc::new(IncludeExcludeKeyFilter::default()),
            Formatting {
                punctuation: Punctuation::test_default(),
            },
        );
        let mut buf = Vec::new();
        formatter.format_record(&mut buf, rec);
        Ok(String::from_utf8(buf)?)
    }

    fn json_raw_value(s: &str) -> Box<json::value::RawValue> {
        json::value::RawValue::from_string(s.into()).unwrap()
    }

    #[test]
    fn test_nested_objects() {
        assert_eq!(
            format(&Record {
                ts: Some(Timestamp::new("2000-01-02T03:04:05.123Z", None)),
                message: Some(RawValue::Json(&json_raw_value(r#""tm""#))),
                level: Some(Level::Debug),
                logger: Some("tl"),
                caller: Some(Caller::Text("tc")),
                fields: RecordFields{
                    head: heapless::Vec::from_slice(&[
                        ("ka", RawValue::Json(&json_raw_value(r#"{"va":{"kb":42}}"#))),
                    ]).unwrap(),
                    tail: Vec::default(),
                },
                predefined: heapless::Vec::default(),
            }).unwrap(),
            String::from("\u{1b}[0;2;3m00-01-02 03:04:05.123 \u{1b}[0;36m|\u{1b}[0;95mDBG\u{1b}[0;36m|\u{1b}[0;2;3m \u{1b}[0;2;4mtl:\u{1b}[0;2;3m \u{1b}[0;1;39mtm \u{1b}[0;32mka\u{1b}[0;2m:\u{1b}[0;33m{ \u{1b}[0;32mva\u{1b}[0;2m:\u{1b}[0;33m{ \u{1b}[0;32mkb\u{1b}[0;2m:\u{1b}[0;94m42\u{1b}[0;33m } }\u{1b}[0;2;3m @ tc\u{1b}[0m"),
        );
    }
}
