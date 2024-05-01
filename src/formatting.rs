// std imports
use std::sync::Arc;

// local imports
use crate::{
    datefmt::DateTimeFormatter,
    filtering::IncludeExcludeSetting,
    fmtx::{aligned_left, centered, Push},
    model::{self, Caller, Level, RawValue},
    settings::Formatting,
    theme::{Element, StylingPush, Theme},
    IncludeExcludeKeyFilter,
};
use encstr::EncodedString;

// relative imports
use string::{Format, MessageFormatAuto, ValueFormatAuto};

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
                if !self.hide_empty_fields || !v.is_empty() {
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
        match value {
            RawValue::String(value) => {
                s.element(Element::Message, |s| {
                    s.batch(|buf| buf.with_auto_trim(|buf| MessageFormatAuto::new(value).format(buf).unwrap()))
                });
            }
            RawValue::Number(value) => {
                s.element(Element::Number, |s| s.batch(|buf| buf.extend(value.as_bytes())));
            }
            RawValue::Boolean(true) => {
                s.element(Element::Boolean, |s| s.batch(|buf| buf.extend(b"true")));
            }
            RawValue::Boolean(false) => {
                s.element(Element::Boolean, |s| s.batch(|buf| buf.extend(b"false")));
            }
            RawValue::Null => {
                s.element(Element::Boolean, |s| s.batch(|buf| buf.extend(b"null")));
            }
            RawValue::Object(value) => {
                s.element(Element::Object, |s| {
                    let item = value.parse().unwrap();
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
            RawValue::Array(value) => {
                let item = value.parse::<256>().unwrap();
                let is_byte_string = item
                    .iter()
                    .map(|&v| v.is_byte_code())
                    .position(|x| x == false)
                    .is_none();
                if is_byte_string {
                    s.batch(|buf| buf.extend(b"b'"));
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
                                s.batch(|buf| buf.extend(self.cfg.punctuation.array_separator.as_bytes()));
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
            s.batch(|buf| key.key_prettify(buf));
        });
        s.element(Element::Field, |s| {
            s.batch(|buf| buf.extend_from_slice(self.rf.cfg.punctuation.field_key_value_separator.as_bytes()));
        });
        if self.rf.unescape_fields {
            self.format_value(s, value, filter, setting);
        } else {
            s.element(Element::String, |s| {
                s.batch(|buf| buf.extend(value.raw_str().as_bytes()))
            });
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
        let value = match value {
            RawValue::String(EncodedString::Raw(value)) => RawValue::auto(value.as_str()),
            _ => value,
        };
        match value {
            RawValue::String(value) => {
                s.element(Element::String, |s| {
                    s.batch(|buf| buf.with_auto_trim(|buf| ValueFormatAuto::new(value).format(buf).unwrap()))
                });
            }
            RawValue::Number(value) => {
                s.element(Element::Number, |s| s.batch(|buf| buf.extend(value.as_bytes())));
            }
            RawValue::Boolean(true) => {
                s.element(Element::Boolean, |s| s.batch(|buf| buf.extend(b"true")));
            }
            RawValue::Boolean(false) => {
                s.element(Element::Boolean, |s| s.batch(|buf| buf.extend(b"false")));
            }
            RawValue::Null => {
                s.element(Element::Null, |s| s.batch(|buf| buf.extend(b"null")));
            }
            RawValue::Object(value) => {
                let item = value.parse().unwrap();
                s.element(Element::Object, |s| {
                    s.batch(|buf| buf.push(b'{'));
                    let mut some_fields_hidden = false;
                    for (k, v) in item.fields.iter() {
                        some_fields_hidden |= !self.format(s, k, *v, filter, setting);
                    }
                    if some_fields_hidden {
                        s.element(Element::Ellipsis, |s| {
                            s.batch(|buf| buf.extend(self.rf.cfg.punctuation.hidden_fields_indicator.as_bytes()))
                        });
                    }
                    s.batch(|buf| {
                        if item.fields.len() != 0 {
                            buf.push(b' ');
                        }
                        buf.push(b'}');
                    });
                });
            }
            RawValue::Array(value) => {
                s.element(Element::Array, |s| {
                    let item = value.parse::<32>().unwrap();
                    s.batch(|buf| buf.push(b'['));
                    let mut first = true;
                    for v in item.iter() {
                        if !first {
                            s.batch(|buf| buf.extend(self.rf.cfg.punctuation.array_separator.as_bytes()));
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

// ---

pub trait WithAutoTrim {
    fn with_auto_trim<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self);
}

impl WithAutoTrim for Vec<u8> {
    #[inline(always)]
    fn with_auto_trim<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let begin = self.len();
        f(self);
        if let Some(end) = self[begin..].iter().rposition(|&b| !b.is_ascii_whitespace()) {
            self.truncate(begin + end + 1);
        }
    }
}

// ---

pub trait KeyPrettify {
    fn key_prettify<B: Push<u8>>(&self, buf: &mut B);
}

impl KeyPrettify for str {
    #[inline(always)]
    fn key_prettify<B: Push<u8>>(&self, buf: &mut B) {
        let bytes = self.as_bytes();
        let mut i = 0;
        while let Some(pos) = bytes[i..].iter().position(|&b| b == b'_') {
            buf.extend_from_slice(&bytes[i..i + pos]);
            buf.push(b'-');
            i += pos + 1;
        }
        buf.extend_from_slice(&bytes[i..])
    }
}

// ---

const HEXDIGIT: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

pub mod string {
    // workspace imports
    use encstr::{AnyEncodedString, Appender, Result};

    // third-party imports
    use bitmask::bitmask;

    // ---

    pub trait Format {
        fn format(&self, buf: &mut Vec<u8>) -> Result<()>;
    }

    // ---

    pub struct ValueFormatAuto<S> {
        string: S,
    }

    impl<S> ValueFormatAuto<S> {
        #[inline(always)]
        pub fn new(string: S) -> Self {
            Self { string }
        }
    }

    impl<'a, S> Format for ValueFormatAuto<S>
    where
        S: AnyEncodedString<'a> + Clone + Copy,
    {
        #[inline(always)]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            use CharGroup::*;

            if self.string.is_empty() {
                buf.extend(r#""""#.as_bytes());
                return Ok(());
            }

            let begin = buf.len();
            ValueFormatRaw::new(self.string).format(buf)?;

            let mut groups: CharGroups = CharGroups::none();

            buf[begin..].iter().map(|&c| CHAR_GROUPS[c as usize]).for_each(|group| {
                groups.set(group);
            });

            let first = buf[begin];
            if groups.is_none() && first != b'[' && first != b'{' {
                return Ok(());
            }

            buf.truncate(begin);

            if groups & (DoubleQuote | SingleQuote | ExtendedSpace | Control) == DoubleQuote.into() {
                return ValueFormatSingleQuoted::new(self.string).format(buf);
            }

            let mask = groups & (DoubleQuote | SingleQuote | ExtendedSpace | Control | Backtick);
            if mask.intersects(DoubleQuote | SingleQuote | ExtendedSpace) && !mask.intersects(Control | Backtick) {
                return ValueFormatBacktickQuoted::new(self.string).format(buf);
            }

            ValueFormatDoubleQuoted::new(self.string).format(buf)
        }
    }

    // ---

    pub struct ValueFormatRaw<S> {
        string: S,
    }

    impl<S> ValueFormatRaw<S> {
        #[inline(always)]
        pub fn new(string: S) -> Self {
            Self { string }
        }
    }

    impl<'a, S> Format for ValueFormatRaw<S>
    where
        S: AnyEncodedString<'a>,
    {
        #[inline(always)]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            self.string.decode(buf)
        }
    }

    // ---

    pub struct ValueFormatDoubleQuoted<S> {
        string: S,
    }

    impl<S> ValueFormatDoubleQuoted<S> {
        #[inline(always)]
        pub fn new(string: S) -> Self {
            Self { string }
        }
    }

    impl<'a, S> Format for ValueFormatDoubleQuoted<S>
    where
        S: AnyEncodedString<'a>,
    {
        #[inline]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            if self.string.source().len() == 0 {
                buf.push(b'"');
            } else if self.string.source().as_bytes()[0] == b'"' {
                buf.extend(self.string.source().as_bytes());
            } else {
                buf.push(b'"');
                self.string.decode(Appender::new(buf))?;
                buf.push(b'"');
            }
            Ok(())
        }
    }

    // ---

    pub struct ValueFormatSingleQuoted<S> {
        string: S,
    }

    impl<S> ValueFormatSingleQuoted<S> {
        #[inline(always)]
        pub fn new(string: S) -> Self {
            Self { string }
        }
    }

    impl<'a, S> Format for ValueFormatSingleQuoted<S>
    where
        S: AnyEncodedString<'a>,
    {
        #[inline(always)]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            buf.push(b'\'');
            self.string.decode(Appender::new(buf))?;
            buf.push(b'\'');
            Ok(())
        }
    }

    // ---

    pub struct ValueFormatBacktickQuoted<S> {
        string: S,
    }

    impl<S> ValueFormatBacktickQuoted<S> {
        #[inline(always)]
        pub fn new(string: S) -> Self {
            Self { string }
        }
    }

    impl<'a, S> Format for ValueFormatBacktickQuoted<S>
    where
        S: AnyEncodedString<'a>,
    {
        #[inline(always)]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            buf.push(b'`');
            self.string.decode(Appender::new(buf))?;
            buf.push(b'`');
            Ok(())
        }
    }

    // ---

    pub struct MessageFormatAuto<S> {
        string: S,
    }

    impl<S> MessageFormatAuto<S> {
        #[inline(always)]
        pub fn new(string: S) -> Self {
            Self { string }
        }
    }

    impl<'a, S> Format for MessageFormatAuto<S>
    where
        S: AnyEncodedString<'a> + Clone + Copy,
    {
        #[inline(always)]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            if self.string.is_empty() {
                return Ok(());
            }

            let begin = buf.len();
            MessageFormatRaw::new(self.string).format(buf)?;
            if buf[begin..].starts_with(b"\"") || buf[begin..].contains(&b'=') {
                buf.truncate(begin);
                MessageFormatDoubleQuoted::new(self.string).format(buf)?;
            }
            Ok(())
        }
    }

    // ---

    pub struct MessageFormatRaw<S> {
        string: S,
    }

    impl<S> MessageFormatRaw<S> {
        #[inline(always)]
        pub fn new(string: S) -> Self {
            Self { string }
        }
    }

    impl<'a, S> Format for MessageFormatRaw<S>
    where
        S: AnyEncodedString<'a>,
    {
        #[inline(always)]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            self.string.decode(buf)
        }
    }

    // ---

    pub struct MessageFormatDoubleQuoted<S> {
        string: S,
    }

    impl<S> MessageFormatDoubleQuoted<S> {
        #[inline(always)]
        pub fn new(string: S) -> Self {
            Self { string }
        }
    }

    impl<'a, S> Format for MessageFormatDoubleQuoted<S>
    where
        S: AnyEncodedString<'a>,
    {
        #[inline]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            if self.string.source().len() == 0 {
                buf.push(b'"');
            } else if self.string.source().as_bytes()[0] == b'"' {
                buf.extend(self.string.source().as_bytes());
            } else {
                buf.push(b'"');
                self.string.decode(Appender::new(buf))?;
                buf.push(b'"');
            }
            Ok(())
        }
    }

    // ---

    static CHAR_GROUPS: [CharGroup; 256] = {
        const CT: CharGroup = CharGroup::Control; // 0x00..0x1F
        const DQ: CharGroup = CharGroup::DoubleQuote; // 0x22
        const SQ: CharGroup = CharGroup::SingleQuote; // 0x27
        const BS: CharGroup = CharGroup::Backslash; // 0x5C
        const BT: CharGroup = CharGroup::Backtick; // 0x60
        const SP: CharGroup = CharGroup::Space; // 0x20
        const XS: CharGroup = CharGroup::ExtendedSpace; // 0x09, 0x0A, 0x0D
        const EQ: CharGroup = CharGroup::EqualSign; // 0x3D
        const __: CharGroup = CharGroup::Nothing;
        [
            //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
            CT, CT, CT, CT, CT, CT, CT, CT, CT, XS, XS, CT, CT, XS, CT, CT, // 0
            CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 1
            SP, __, DQ, __, __, __, __, SQ, __, __, __, __, __, __, __, __, // 2
            __, __, __, __, __, __, __, __, __, __, __, __, __, EQ, __, __, // 3
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
            __, __, __, __, __, __, __, __, __, __, __, __, BS, __, __, __, // 5
            BT, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
        ]
    };

    bitmask! {
        #[derive(Debug)]
        pub mask CharGroups: u8 where flags CharGroup {
            Nothing       = 0b00000000,
            Control       = 0b00000001,
            DoubleQuote   = 0b00000010,
            SingleQuote   = 0b00000100,
            Backslash     = 0b00001000,
            Backtick      = 0b00010000,
            Space         = 0b00100000,
            ExtendedSpace = 0b01000000,
            EqualSign     = 0b10000000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        datefmt::LinuxDateFormat,
        error::Error,
        model::{RawObject, Record, RecordFields},
        settings::Punctuation,
        theme::Theme,
        themecfg::testing,
        timestamp::Timestamp,
        timezone::Tz,
    };
    use chrono::{Offset, Utc};
    use encstr::EncodedString;
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
                ts: Some(Timestamp::new("2000-01-02T03:04:05.123Z")),
                message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
                level: Some(Level::Debug),
                logger: Some("tl"),
                caller: Some(Caller::Text("tc")),
                fields: RecordFields{
                    head: heapless::Vec::from_slice(&[
                        ("ka", RawValue::from(RawObject::Json(&json_raw_value(r#"{"va":{"kb":42}}"#)))),
                    ]).unwrap(),
                    tail: Vec::default(),
                },
                predefined: heapless::Vec::default(),
            }).unwrap(),
            String::from("\u{1b}[0;2;3m00-01-02 03:04:05.123 \u{1b}[0;36m|\u{1b}[0;95mDBG\u{1b}[0;36m|\u{1b}[0;2;3m \u{1b}[0;2;4mtl:\u{1b}[0;2;3m \u{1b}[0;1;39mtm \u{1b}[0;32mka\u{1b}[0;2m:\u{1b}[0;33m{ \u{1b}[0;32mva\u{1b}[0;2m:\u{1b}[0;33m{ \u{1b}[0;32mkb\u{1b}[0;2m:\u{1b}[0;94m42\u{1b}[0;33m } }\u{1b}[0;2;3m @ tc\u{1b}[0m"),
        );
    }
}
