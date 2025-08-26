// std imports
use std::sync::Arc;

// workspace imports
use encstr::EncodedString;

// local imports
use crate::{
    IncludeExcludeKeyFilter,
    datefmt::DateTimeFormatter,
    filtering::IncludeExcludeSetting,
    fmtx::{OptimizedBuf, Push, aligned_left, centered},
    model::{self, Level, RawValue},
    settings::{AsciiMode, Formatting, Punctuation},
    theme::{Element, StylingPush, Theme},
};

// relative imports
use string::{DynMessageFormat, Format, ValueFormatAuto};

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

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoOpRecordWithSourceFormatter;

impl RecordWithSourceFormatter for NoOpRecordWithSourceFormatter {
    #[inline(always)]
    fn format_record(&self, _: &mut Buf, _: model::RecordWithSource) {}
}

// ---

#[derive(Default, Clone)]
pub struct RecordFormatterBuilder {
    theme: Option<Arc<Theme>>,
    raw_fields: bool,
    ts_formatter: Option<Arc<DateTimeFormatter>>,
    hide_empty_fields: bool,
    flatten: bool,
    ascii: AsciiMode,
    always_show_time: bool,
    always_show_level: bool,
    fields: Option<Arc<IncludeExcludeKeyFilter>>,
    cfg: Option<Formatting>,
    punctuation: Option<Arc<Punctuation<String>>>,
}

impl RecordFormatterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_raw_fields(self, enabled: bool) -> Self {
        Self {
            raw_fields: enabled,
            ..self
        }
    }

    pub fn with_empty_fields_hiding(self, enabled: bool) -> Self {
        Self {
            hide_empty_fields: enabled,
            ..self
        }
    }

    pub fn with_flatten(self, flatten: bool) -> Self {
        Self { flatten, ..self }
    }

    pub fn with_ascii(self, ascii: AsciiMode) -> Self {
        Self { ascii, ..self }
    }

    pub fn with_always_show_time(self, value: bool) -> Self {
        Self {
            always_show_time: value,
            ..self
        }
    }

    pub fn with_always_show_level(self, value: bool) -> Self {
        Self {
            always_show_level: value,
            ..self
        }
    }

    pub fn with_theme(self, value: Arc<Theme>) -> Self {
        Self {
            theme: Some(value),
            ..self
        }
    }

    pub fn with_timestamp_formatter(self, value: Arc<DateTimeFormatter>) -> Self {
        Self {
            ts_formatter: Some(value),
            ..self
        }
    }

    pub fn with_options(self, value: Formatting) -> Self {
        Self {
            cfg: Some(value),
            ..self
        }
    }

    pub fn with_field_filter(self, value: Arc<IncludeExcludeKeyFilter>) -> Self {
        Self {
            fields: Some(value),
            ..self
        }
    }

    pub fn with_punctuation(self, value: Arc<Punctuation<String>>) -> Self {
        Self {
            punctuation: Some(value),
            ..self
        }
    }

    pub fn build(self) -> RecordFormatter {
        let cfg = self.cfg.unwrap_or_default();
        let message_format = (&cfg, self.ascii).into();
        let punctuation = self
            .punctuation
            .unwrap_or_else(|| cfg.punctuation.resolve(self.ascii).into());
        let ts_formatter = self.ts_formatter.unwrap_or_default();
        let ts_width = ts_formatter.max_length();

        RecordFormatter {
            theme: self.theme.unwrap_or_default(),
            unescape_fields: !self.raw_fields,
            ts_formatter,
            ts_width,
            hide_empty_fields: self.hide_empty_fields,
            flatten: self.flatten,
            always_show_time: self.always_show_time,
            always_show_level: self.always_show_level,
            fields: self.fields.unwrap_or_default(),
            message_format,
            punctuation,
        }
    }
}

pub struct RecordFormatter {
    theme: Arc<Theme>,
    unescape_fields: bool,
    ts_formatter: Arc<DateTimeFormatter>,
    ts_width: usize,
    hide_empty_fields: bool,
    flatten: bool,
    always_show_time: bool,
    always_show_level: bool,
    fields: Arc<IncludeExcludeKeyFilter>,
    message_format: DynMessageFormat,
    punctuation: Arc<Punctuation<String>>,
}

impl RecordFormatter {
    pub fn format_record(&self, buf: &mut Buf, rec: &model::Record) {
        let mut fs = FormattingState::new(self.flatten && self.unescape_fields);

        self.theme.apply(buf, &rec.level, |s| {
            //
            // time
            //
            if let Some(ts) = &rec.ts {
                fs.add_element(|| {});
                s.element(Element::Time, |s| {
                    s.batch(|buf| {
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
                    })
                });
            } else if self.always_show_time {
                fs.add_element(|| {});
                s.element(Element::Time, |s| {
                    s.batch(|buf| {
                        centered(buf, self.ts_width, b'-', |mut buf| {
                            buf.extend_from_slice(b"-");
                        });
                    })
                });
            }

            //
            // level
            //
            let level = match rec.level {
                Some(Level::Error) => Some(b"ERR"),
                Some(Level::Warning) => Some(b"WRN"),
                Some(Level::Info) => Some(b"INF"),
                Some(Level::Debug) => Some(b"DBG"),
                Some(Level::Trace) => Some(b"TRC"),
                None => None,
            };
            let level = level.or_else(|| self.always_show_level.then(|| b"(?)"));
            if let Some(level) = level {
                fs.add_element(|| s.space());
                s.element(Element::Level, |s| {
                    s.batch(|buf| {
                        buf.extend_from_slice(self.punctuation.level_left_separator.as_bytes());
                    });
                    s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(level)));
                    s.batch(|buf| buf.extend_from_slice(self.punctuation.level_right_separator.as_bytes()));
                });
            }

            //
            // logger
            //
            if let Some(logger) = rec.logger {
                fs.add_element(|| s.batch(|buf| buf.push(b' ')));
                s.element(Element::Logger, |s| {
                    s.element(Element::LoggerInner, |s| {
                        s.batch(|buf| buf.extend_from_slice(logger.as_bytes()))
                    });
                    s.batch(|buf| buf.extend_from_slice(self.punctuation.logger_name_separator.as_bytes()));
                });
            }
            //
            // message text
            //
            if let Some(value) = &rec.message {
                self.format_message(s, &mut fs, *value);
            } else {
                s.reset();
            }
            //
            // fields
            //
            let mut some_fields_hidden = false;
            for (k, v) in rec.fields() {
                if !self.hide_empty_fields || !v.is_empty() {
                    some_fields_hidden |= !self.format_field(s, k, *v, &mut fs, Some(&self.fields));
                }
            }
            if some_fields_hidden || (fs.some_nested_fields_hidden && fs.flatten) {
                s.element(Element::Ellipsis, |s| {
                    s.batch(|buf| buf.extend_from_slice(self.punctuation.hidden_fields_indicator.as_bytes()))
                });
            }
            //
            // caller
            //
            if !rec.caller.is_empty() {
                let caller = rec.caller;
                s.element(Element::Caller, |s| {
                    s.batch(|buf| {
                        buf.push(b' ');
                        buf.extend(self.punctuation.source_location_separator.as_bytes())
                    });
                    s.element(Element::CallerInner, |s| {
                        s.batch(|buf| {
                            if !caller.name.is_empty() {
                                buf.extend(caller.name.as_bytes());
                            }
                            if !caller.file.is_empty() || !caller.line.is_empty() {
                                if !caller.name.is_empty() {
                                    buf.extend(self.punctuation.caller_name_file_separator.as_bytes());
                                }
                                buf.extend(caller.file.as_bytes());
                                if caller.line.len() != 0 {
                                    buf.push(b':');
                                    buf.extend(caller.line.as_bytes());
                                }
                            }
                        });
                    });
                });
            };
        });
    }

    #[inline]
    fn format_field<'a, S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        key: &str,
        value: RawValue<'a>,
        fs: &mut FormattingState,
        filter: Option<&IncludeExcludeKeyFilter>,
    ) -> bool {
        let mut fv = FieldFormatter::new(self);
        fv.format(s, key, value, fs, filter, IncludeExcludeSetting::Unspecified)
    }

    #[inline]
    fn format_message<'a, S: StylingPush<Buf>>(&self, s: &mut S, fs: &mut FormattingState, value: RawValue<'a>) {
        match value {
            RawValue::String(value) => {
                if !value.is_empty() {
                    fs.add_element(|| {
                        s.reset();
                        s.space();
                    });
                    s.element(Element::Message, |s| {
                        s.batch(|buf| self.message_format.format(value, buf).unwrap())
                    });
                }
                false
            }
            _ => self.format_field(s, "msg", value, fs, Some(self.fields.as_ref())),
        };
    }
}

impl RecordWithSourceFormatter for RecordFormatter {
    #[inline]
    fn format_record(&self, buf: &mut Buf, rec: model::RecordWithSource) {
        RecordFormatter::format_record(self, buf, rec.record)
    }
}

// ---

struct FormattingState {
    key_prefix: KeyPrefix,
    flatten: bool,
    empty: bool,
    some_nested_fields_hidden: bool,
    has_fields: bool,
}

impl FormattingState {
    #[inline]
    fn new(flatten: bool) -> Self {
        Self {
            key_prefix: KeyPrefix::default(),
            flatten,
            empty: true,
            some_nested_fields_hidden: false,
            has_fields: false,
        }
    }

    fn add_element(&mut self, add_space: impl FnOnce()) {
        if self.empty {
            self.empty = false;
        } else {
            add_space();
        }
    }
}

// ---

#[derive(Default)]
struct KeyPrefix {
    value: OptimizedBuf<u8, 256>,
}

impl KeyPrefix {
    #[inline]
    fn len(&self) -> usize {
        self.value.len()
    }

    #[inline]
    fn format<B: Push<u8>>(&self, buf: &mut B) {
        buf.extend_from_slice(&self.value.as_slices().0);
        buf.extend_from_slice(&self.value.as_slices().1);
    }

    #[inline]
    fn push(&mut self, key: &str) -> usize {
        let len = self.len();
        if len != 0 {
            self.value.push(b'.');
        }
        key.key_prettify(&mut self.value);
        self.len() - len
    }

    #[inline]
    fn pop(&mut self, n: usize) {
        if n != 0 {
            let len = self.len();
            if n >= len {
                self.value.clear();
            } else {
                self.value.truncate(len - n);
            }
        }
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
        fs: &mut FormattingState,
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
        let ffv = self.begin(s, key, value, fs);
        if self.rf.unescape_fields {
            self.format_value(s, value, fs, filter, setting);
        } else {
            s.element(Element::String, |s| {
                s.batch(|buf| buf.extend(value.raw_str().as_bytes()))
            });
        }
        self.end(fs, ffv);
        true
    }

    fn format_value<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        value: RawValue<'a>,
        fs: &mut FormattingState,
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
                    s.batch(|buf| ValueFormatAuto.format(value, buf).unwrap())
                });
            }
            RawValue::Number(value) => {
                s.element(Element::Number, |s| s.batch(|buf| buf.extend(value.as_bytes())));
            }
            RawValue::Boolean(true) => {
                s.element(Element::BooleanTrue, |s| s.batch(|buf| buf.extend(b"true")));
            }
            RawValue::Boolean(false) => {
                s.element(Element::BooleanFalse, |s| s.batch(|buf| buf.extend(b"false")));
            }
            RawValue::Null => {
                s.element(Element::Null, |s| s.batch(|buf| buf.extend(b"null")));
            }
            RawValue::Object(value) => {
                let mut item = model::Object::default();
                value.parse_into(&mut item).ok();
                s.element(Element::Object, |s| {
                    if !fs.flatten {
                        s.batch(|buf| buf.push(b'{'));
                    }
                    let mut some_fields_hidden = false;
                    for (k, v) in item.fields.iter() {
                        some_fields_hidden |= !self.format(s, k, *v, fs, filter, setting);
                    }
                    if !fs.flatten {
                        if some_fields_hidden {
                            s.element(Element::Ellipsis, |s| {
                                s.batch(|buf| buf.extend(self.rf.punctuation.hidden_fields_indicator.as_bytes()))
                            });
                        }
                        s.batch(|buf| {
                            if item.fields.len() != 0 {
                                buf.push(b' ');
                            }
                            buf.push(b'}');
                        });
                    }
                    fs.some_nested_fields_hidden |= some_fields_hidden;
                });
            }
            RawValue::Array(value) => {
                s.element(Element::Array, |s| {
                    let mut item = model::Array::default();
                    value.parse_into::<32>(&mut item).ok();
                    s.batch(|buf| buf.push(b'['));
                    let mut first = true;
                    for v in item.iter() {
                        if !first {
                            s.batch(|buf| buf.extend(self.rf.punctuation.array_separator.as_bytes()));
                        } else {
                            first = false;
                        }
                        self.format_value(s, *v, fs, None, IncludeExcludeSetting::Unspecified);
                    }
                    s.batch(|buf| buf.push(b']'));
                });
            }
        };
    }

    #[inline(always)]
    fn begin<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        key: &str,
        value: RawValue<'a>,
        fs: &mut FormattingState,
    ) -> FormattedFieldVariant {
        if fs.flatten && matches!(value, RawValue::Object(_)) {
            return FormattedFieldVariant::Flattened(fs.key_prefix.push(key));
        }

        if !fs.has_fields {
            fs.has_fields = true;
            if self.rf.message_format.delimited {
                fs.add_element(|| s.space());
                s.element(Element::MessageDelimiter, |s| {
                    s.batch(|buf| buf.extend(self.rf.punctuation.message_delimiter.as_bytes()));
                });
            }
        }

        let variant = FormattedFieldVariant::Normal { flatten: fs.flatten };

        fs.add_element(|| s.space());
        s.element(Element::Key, |s| {
            s.batch(|buf| {
                if fs.flatten {
                    fs.flatten = false;
                    if fs.key_prefix.len() != 0 {
                        fs.key_prefix.format(buf);
                        buf.push(b'.');
                    }
                }
                key.key_prettify(buf);
            });
        });
        s.element(Element::Field, |s| {
            s.batch(|buf| buf.extend(self.rf.punctuation.field_key_value_separator.as_bytes()));
        });

        variant
    }

    #[inline]
    fn end(&mut self, fs: &mut FormattingState, v: FormattedFieldVariant) {
        match v {
            FormattedFieldVariant::Normal { flatten } => {
                fs.flatten = flatten;
            }
            FormattedFieldVariant::Flattened(n) => {
                fs.key_prefix.pop(n);
            }
        }
    }
}

// ---

pub trait WithAutoTrim {
    fn with_auto_trim<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R;
}

impl WithAutoTrim for Vec<u8> {
    #[inline(always)]
    fn with_auto_trim<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let begin = self.len();
        let result = f(self);
        if let Some(end) = self[begin..].iter().rposition(|&b| !b.is_ascii_whitespace()) {
            self.truncate(begin + end + 1);
        } else {
            self.truncate(begin);
        }
        result
    }
}

// ---

trait KeyPrettify {
    fn key_prettify<B: Push<u8>>(&self, buf: &mut B);
}

impl KeyPrettify for str {
    #[inline]
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

enum FormattedFieldVariant {
    Normal { flatten: bool },
    Flattened(usize),
}

// ---

pub mod string {
    // std imports
    use std::{cmp::min, sync::Arc};

    // third-party imports
    use enumset::{EnumSet, EnumSetType, enum_set as mask};

    // workspace imports
    use encstr::{AnyEncodedString, EncodedString, JsonAppender, Result};
    use enumset_ext::EnumSetExt;

    // local imports
    use crate::{
        formatting::WithAutoTrim,
        model::{MAX_NUMBER_LEN, looks_like_number},
        settings::MessageFormat,
    };

    // ---

    pub trait Format {
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()>;

        fn rtrim(self, n: usize) -> FormatRightTrimmed<Self>
        where
            Self: Sized,
        {
            FormatRightTrimmed::new(n, self)
        }
    }

    pub type DynFormat = Arc<dyn Format>;

    #[derive(Clone)]
    pub struct DynMessageFormat {
        format: DynFormat,
        pub delimited: bool,
    }

    impl std::ops::Deref for DynMessageFormat {
        type Target = DynFormat;

        fn deref(&self) -> &Self::Target {
            &self.format
        }
    }

    impl From<(&super::Formatting, super::AsciiMode)> for DynMessageFormat {
        fn from((formatting, ascii): (&super::Formatting, super::AsciiMode)) -> Self {
            new_message_format(
                formatting.message.format,
                &formatting.punctuation.message_delimiter.resolve(ascii),
            )
        }
    }

    pub fn new_message_format(setting: MessageFormat, delimiter: &str) -> DynMessageFormat {
        let (format, delimited): (DynFormat, _) = match setting {
            MessageFormat::AutoQuoted => (Arc::new(MessageFormatAutoQuoted), false),
            MessageFormat::AlwaysQuoted => (Arc::new(MessageFormatAlwaysQuoted), false),
            MessageFormat::AlwaysDoubleQuoted => (Arc::new(MessageFormatDoubleQuoted), false),
            MessageFormat::Delimited => {
                let delimiter = format!(" {} ", delimiter);
                let n = delimiter.len();
                (Arc::new(MessageFormatDelimited::new(delimiter).rtrim(n)), true)
            }
            MessageFormat::Raw => (Arc::new(MessageFormatRaw), false),
        };
        DynMessageFormat { format, delimited }
    }

    // ---

    pub struct ValueFormatAuto;

    impl Format for ValueFormatAuto {
        #[inline(always)]
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()> {
            if input.is_empty() {
                buf.extend(r#""""#.as_bytes());
                return Ok(());
            }

            let begin = buf.len();
            buf.with_auto_trim(|buf| ValueFormatRaw.format(input, buf))?;

            let mut mask = Mask::empty();

            buf[begin..].iter().map(|&c| CHAR_GROUPS[c as usize]).for_each(|group| {
                mask |= group;
            });

            let plain = if (mask & !(Flag::Other | Flag::Digit | Flag::Dot | Flag::Minus)).is_empty() {
                if mask == Flag::Digit {
                    buf[begin..].len() > MAX_NUMBER_LEN
                } else if !mask.contains(Flag::Other) {
                    !looks_like_number(&buf[begin..])
                } else {
                    !matches!(
                        buf[begin..],
                        [b'{', ..]
                            | [b'[', ..]
                            | [b't', b'r', b'u', b'e']
                            | [b'f', b'a', b'l', b's', b'e']
                            | [b'n', b'u', b'l', b'l']
                    )
                }
            } else {
                false
            };

            if plain {
                return Ok(());
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            const Z: Mask = Mask::empty();
            const XS: Mask = mask!(Flag::Control | Flag::ExtendedSpace);

            if matches!(mask & (Flag::Backtick | XS), Z | XS) {
                buf.push(b'`');
                buf.push(b'`');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            buf.truncate(begin);
            ValueFormatDoubleQuoted.format(input, buf)
        }
    }

    // ---

    pub struct ValueFormatRaw;

    impl Format for ValueFormatRaw {
        #[inline(always)]
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()> {
            input.decode(buf)
        }
    }

    // ---

    pub struct ValueFormatDoubleQuoted;

    impl Format for ValueFormatDoubleQuoted {
        #[inline]
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()> {
            input.format_json(buf)
        }
    }

    // ---

    pub struct MessageFormatAutoQuoted;

    impl Format for MessageFormatAutoQuoted {
        #[inline(always)]
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()> {
            if input.is_empty() {
                return Ok(());
            }

            let begin = buf.len();
            buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf))?;

            let mut mask = Mask::empty();

            buf[begin..].iter().map(|&c| CHAR_GROUPS[c as usize]).for_each(|group| {
                mask |= group;
            });

            if !mask.intersects(Flag::EqualSign | Flag::Control | Flag::Backslash)
                && !matches!(buf[begin..], [b'"', ..] | [b'\'', ..] | [b'`', ..])
            {
                return Ok(());
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            const Z: Mask = Mask::empty();
            const XS: Mask = mask!(Flag::Control | Flag::ExtendedSpace);

            if matches!(mask & (Flag::Backtick | XS), Z | XS) {
                buf.push(b'`');
                buf.push(b'`');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            buf.truncate(begin);
            MessageFormatDoubleQuoted.format(input, buf)
        }
    }

    // ---

    pub struct MessageFormatAlwaysQuoted;

    impl Format for MessageFormatAlwaysQuoted {
        #[inline(always)]
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()> {
            if input.is_empty() {
                return Ok(());
            }

            let begin = buf.len();
            buf.push(b'"');
            buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf))?;

            let mut mask = Mask::empty();

            let body = begin + 1;
            buf[body..].iter().map(|&c| CHAR_GROUPS[c as usize]).for_each(|group| {
                mask |= group;
            });

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'"');
                return Ok(());
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Backslash) {
                buf[begin] = b'\'';
                buf.push(b'\'');
                return Ok(());
            }

            const Z: Mask = Mask::empty();
            const XS: Mask = mask!(Flag::Control | Flag::ExtendedSpace);

            if matches!(mask & (Flag::Backtick | XS), Z | XS) {
                buf[begin] = b'`';
                buf.push(b'`');
                return Ok(());
            }

            buf.truncate(begin);
            MessageFormatDoubleQuoted.format(input, buf)
        }
    }

    // ---

    pub struct MessageFormatDelimited(String);

    impl MessageFormatDelimited {
        pub fn new(suffix: String) -> Self {
            Self(suffix)
        }
    }

    impl Format for MessageFormatDelimited {
        #[inline(always)]
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()> {
            if input.is_empty() {
                return Ok(());
            }

            let begin = buf.len();
            buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf))?;

            let mut mask = Mask::empty();

            buf[begin..].iter().map(|&c| CHAR_GROUPS[c as usize]).for_each(|group| {
                mask |= group;
            });

            if !mask.contains(Flag::Control)
                && !matches!(buf[begin..], [b'"', ..] | [b'\'', ..] | [b'`', ..])
                && memchr::memmem::find(&buf[begin..], self.0.as_bytes()).is_none()
            {
                buf.extend(self.0.as_bytes());
                return Ok(());
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                buf.extend(self.0.as_bytes());
                return Ok(());
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                buf.extend(self.0.as_bytes());
                return Ok(());
            }

            const Z: Mask = Mask::empty();
            const XS: Mask = mask!(Flag::Control | Flag::ExtendedSpace);

            if matches!(mask & (Flag::Backtick | XS), Z | XS) {
                buf.push(b'`');
                buf.push(b'`');
                buf[begin..].rotate_right(1);
                buf.extend(self.0.as_bytes());
                return Ok(());
            }

            buf.truncate(begin);
            MessageFormatDoubleQuoted.format(input, buf)?;
            buf.extend(self.0.as_bytes());
            Ok(())
        }
    }

    // ---

    pub struct MessageFormatRaw;

    impl Format for MessageFormatRaw {
        #[inline(always)]
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()> {
            input.decode(buf)
        }
    }

    // ---

    pub struct MessageFormatDoubleQuoted;

    impl Format for MessageFormatDoubleQuoted {
        #[inline]
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()> {
            input.format_json(buf)
        }
    }

    // ---

    pub struct FormatRightTrimmed<F> {
        n: usize,
        inner: F,
    }

    impl<F> FormatRightTrimmed<F> {
        fn new(n: usize, inner: F) -> Self {
            Self { n, inner }
        }
    }

    impl<F: Format> Format for FormatRightTrimmed<F> {
        #[inline]
        fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>) -> Result<()> {
            let begin = buf.len();
            self.inner.format(input, buf)?;
            buf.truncate(buf.len() - min(buf.len() - begin, self.n));
            Ok(())
        }
    }

    // ---

    trait EncodedStringExt {
        fn format_json(&self, buf: &mut Vec<u8>) -> Result<()>;
    }

    impl<'a, S> EncodedStringExt for S
    where
        S: AnyEncodedString<'a>,
    {
        #[inline]
        fn format_json(&self, buf: &mut Vec<u8>) -> Result<()> {
            buf.push(b'"');
            self.decode(JsonAppender::new(buf))?;
            buf.push(b'"');
            Ok(())
        }
    }

    // ---

    static CHAR_GROUPS: [Mask; 256] = {
        const CT: Mask = mask!(Flag::Control); // 0x00..0x1F
        const DQ: Mask = mask!(Flag::DoubleQuote); // 0x22
        const SQ: Mask = mask!(Flag::SingleQuote); // 0x27
        const BS: Mask = mask!(Flag::Backslash); // 0x5C
        const BT: Mask = mask!(Flag::Backtick); // 0x60
        const SP: Mask = mask!(Flag::Space); // 0x20
        const XS: Mask = mask!(Flag::Control | Flag::ExtendedSpace); // 0x09, 0x0A, 0x0D
        const EQ: Mask = mask!(Flag::EqualSign); // 0x3D
        const HY: Mask = mask!(Flag::Minus); // Hyphen, 0x2D
        const DO: Mask = mask!(Flag::Dot); // Dot, 0x2E
        const DD: Mask = mask!(Flag::Digit); // Decimal digit, 0x30..0x39
        const __: Mask = mask!(Flag::Other);
        [
            //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
            CT, CT, CT, CT, CT, CT, CT, CT, CT, XS, XS, CT, CT, XS, CT, CT, // 0
            CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 1
            SP, __, DQ, __, __, __, __, SQ, __, __, __, __, __, HY, DO, __, // 2
            DD, DD, DD, DD, DD, DD, DD, DD, DD, DD, __, __, __, EQ, __, __, // 3
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

    #[derive(EnumSetType)]
    enum Flag {
        Control,
        DoubleQuote,
        SingleQuote,
        Backslash,
        Backtick,
        Space,
        ExtendedSpace,
        EqualSign,
        Digit,
        Minus,
        Dot,
        Other,
    }

    type Mask = EnumSet<Flag>;
}

#[cfg(test)]
mod tests {
    use super::{string::new_message_format, *};
    use crate::{
        datefmt::LinuxDateFormat,
        model::{Caller, RawObject, Record, RecordFields, RecordWithSourceConstructor},
        settings::{AsciiMode, MessageFormat, MessageFormatting, Punctuation},
        timestamp::Timestamp,
        timezone::Tz,
    };
    use chrono::{Offset, Utc};
    use encstr::EncodedString;
    use serde_json as json;

    trait FormatToVec {
        fn format_to_vec(&self, rec: &Record) -> Vec<u8>;
    }

    trait FormatToString {
        fn format_to_string(&self, rec: &Record) -> String;
    }

    impl FormatToVec for RecordFormatter {
        fn format_to_vec(&self, rec: &Record) -> Vec<u8> {
            let mut buf = Vec::new();
            self.format_record(&mut buf, rec);
            buf
        }
    }

    impl FormatToString for RecordFormatter {
        fn format_to_string(&self, rec: &Record) -> String {
            String::from_utf8(self.format_to_vec(rec)).unwrap()
        }
    }

    fn formatter() -> RecordFormatterBuilder {
        RecordFormatterBuilder::new()
            .with_theme(crate::testing::theme())
            .with_timestamp_formatter(
                DateTimeFormatter::new(
                    LinuxDateFormat::new("%y-%m-%d %T.%3N").compile(),
                    Tz::FixedOffset(Utc.fix()),
                )
                .into(),
            )
            .with_options(Formatting {
                flatten: None,
                message: MessageFormatting {
                    format: MessageFormat::AutoQuoted,
                },
                punctuation: Punctuation::test_default(),
            })
    }

    fn format(rec: &Record) -> String {
        formatter().build().format_to_string(rec)
    }

    fn format_no_color(rec: &Record) -> String {
        formatter().with_theme(Default::default()).build().format_to_string(rec)
    }

    fn json_raw_value(s: &str) -> Box<json::value::RawValue> {
        json::value::RawValue::from_string(s.into()).unwrap()
    }

    trait RecordExt<'a> {
        fn from_fields(fields: &[(&'a str, RawValue<'a>)]) -> Record<'a>;
    }

    impl<'a> RecordExt<'a> for Record<'a> {
        fn from_fields(fields: &[(&'a str, RawValue<'a>)]) -> Record<'a> {
            Record {
                fields: RecordFields::from_slice(fields),
                ..Default::default()
            }
        }
    }

    #[test]
    fn test_nested_objects() {
        let ka = json_raw_value(r#"{"va":{"kb":42,"kc":43}}"#);
        let rec = Record {
            ts: Some(Timestamp::new("2000-01-02T03:04:05.123Z")),
            message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
            level: Some(Level::Debug),
            logger: Some("tl"),
            caller: Caller::with_name("tc"),
            fields: RecordFields::from_slice(&[("k_a", RawValue::from(RawObject::Json(&ka)))]),
            ..Default::default()
        };

        assert_eq!(
            &format(&rec),
            "\u{1b}[0;2;3m00-01-02 03:04:05.123 \u{1b}[0;36m|\u{1b}[0;95mDBG\u{1b}[0;36m|\u{1b}[0;2;3m \u{1b}[0;2;4mtl:\u{1b}[0m \u{1b}[0;1;39mtm \u{1b}[0;32mk-a\u{1b}[0;2m=\u{1b}[0;33m{ \u{1b}[0;32mva\u{1b}[0;2m=\u{1b}[0;33m{ \u{1b}[0;32mkb\u{1b}[0;2m=\u{1b}[0;94m42 \u{1b}[0;32mkc\u{1b}[0;2m=\u{1b}[0;94m43\u{1b}[0;33m } }\u{1b}[0;2;3m @ tc\u{1b}[0m",
        );

        assert_eq!(
            &formatter().with_flatten(true).build().format_to_string(&rec),
            "\u{1b}[0;2;3m00-01-02 03:04:05.123 \u{1b}[0;36m|\u{1b}[0;95mDBG\u{1b}[0;36m|\u{1b}[0;2;3m \u{1b}[0;2;4mtl:\u{1b}[0m \u{1b}[0;1;39mtm \u{1b}[0;32mk-a.va.kb\u{1b}[0;2m=\u{1b}[0;94m42 \u{1b}[0;32mk-a.va.kc\u{1b}[0;2m=\u{1b}[0;94m43\u{1b}[0;2;3m @ tc\u{1b}[0m",
        );
    }

    #[test]
    fn test_timestamp_none() {
        let rec = Record {
            message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
            level: Some(Level::Error),
            ..Default::default()
        };

        assert_eq!(&format(&rec), "\u{1b}[0;7;91m|ERR|\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m");
    }

    #[test]
    fn test_level_trace() {
        let rec = Record {
            message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
            level: Some(Level::Trace),
            ..Default::default()
        };

        assert_eq!(
            &format(&rec),
            "\u{1b}[0;36m|\u{1b}[0;2mTRC\u{1b}[0;36m|\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m"
        );
    }

    #[test]
    fn test_timestamp_none_always_show() {
        let rec = Record {
            message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
            ..Default::default()
        };

        assert_eq!(
            &formatter().with_always_show_time(true).build().format_to_string(&rec),
            "\u{1b}[0;2;3m---------------------\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m",
        );
    }

    #[test]
    fn test_level_none() {
        let rec = Record {
            message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
            ..Default::default()
        };

        assert_eq!(&format(&rec), "\u{1b}[0;1;39mtm\u{1b}[0m",);
    }

    #[test]
    fn test_level_none_always_show() {
        let rec = Record {
            message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
            ..Default::default()
        };

        assert_eq!(
            &formatter().with_always_show_level(true).build().format_to_string(&rec),
            "\u{1b}[0;36m|(?)|\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m"
        );
    }

    #[test]
    fn test_string_value_raw() {
        let v = "v";
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(&format_no_color(&rec), "k=v");
    }

    #[test]
    fn test_string_value_json_simple() {
        let v = r#""some-value""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k=some-value"#);
    }

    #[test]
    fn test_string_value_json_space() {
        let v = r#""some value""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k="some value""#);
    }

    #[test]
    fn test_string_value_raw_space() {
        let v = r#"some value"#;
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k="some value""#);
    }

    #[test]
    fn test_string_value_json_space_and_double_quotes() {
        let v = r#""some \"value\"""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k='some "value"'"#);
    }

    #[test]
    fn test_string_value_raw_space_and_double_quotes() {
        let v = r#"some "value""#;
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k='some "value"'"#);
    }

    #[test]
    fn test_string_value_json_space_and_single_quotes() {
        let v = r#""some 'value'""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k="some 'value'""#);
    }

    #[test]
    fn test_string_value_raw_space_and_single_quotes() {
        let v = r#"some 'value'"#;
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k="some 'value'""#);
    }

    #[test]
    fn test_string_value_json_space_and_backticks() {
        let v = r#""some `value`""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k="some `value`""#);
    }

    #[test]
    fn test_string_value_raw_space_and_backticks() {
        let v = r#"some `value`"#;
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k="some `value`""#);
    }

    #[test]
    fn test_string_value_json_space_and_double_and_single_quotes() {
        let v = r#""some \"value\" from 'source'""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k=`some "value" from 'source'`"#);
    }

    #[test]
    fn test_string_value_raw_space_and_double_and_single_quotes() {
        let v = r#"some "value" from 'source'"#;
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k=`some "value" from 'source'`"#);
    }

    #[test]
    fn test_string_value_json_backslash() {
        let v = r#""some-\\\"value\\\"""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k=`some-\"value\"`"#);
    }

    #[test]
    fn test_string_value_raw_backslash() {
        let v = r#"some-\"value\""#;
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(&format_no_color(&rec), r#"k=`some-\"value\"`"#);
    }

    #[test]
    fn test_string_value_json_space_and_double_and_single_quotes_and_backticks() {
        let v = r#""some \"value\" from 'source' with `sauce`""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(
            &format_no_color(&rec),
            r#"k="some \"value\" from 'source' with `sauce`""#
        );
    }

    #[test]
    fn test_string_value_raw_space_and_double_and_single_quotes_and_backticks() {
        let v = r#"some "value" from 'source' with `sauce`"#;
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(
            &format_no_color(&rec),
            r#"k="some \"value\" from 'source' with `sauce`""#
        );
    }

    #[test]
    fn test_string_value_json_extended_space() {
        let v = r#""some\tvalue""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(&format_no_color(&rec), "k=`some\tvalue`");
    }

    #[test]
    fn test_string_value_raw_extended_space() {
        let v = "some\tvalue";
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(&format_no_color(&rec), "k=`some\tvalue`");
    }

    #[test]
    fn test_string_value_json_control_chars() {
        let v = r#""some-\u001b[1mvalue\u001b[0m""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }

    #[test]
    fn test_string_value_raw_control_chars() {
        let rec = Record::from_fields(&[("k", EncodedString::raw("some-\x1b[1mvalue\x1b[0m").into())]);

        let result = format_no_color(&rec);
        assert_eq!(&result, r#"k="some-\u001b[1mvalue\u001b[0m""#, "{}", result);
    }

    #[test]
    fn test_string_value_json_control_chars_and_quotes() {
        let v = r#""some-\u001b[1m\"value\"\u001b[0m""#;
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }

    #[test]
    fn test_string_value_raw_control_chars_and_quotes() {
        let v = "some-\x1b[1m\"value\"\x1b[0m";
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(format_no_color(&rec), r#"k="some-\u001b[1m\"value\"\u001b[0m""#);
    }

    #[test]
    fn test_string_value_json_ambiguous() {
        for v in ["true", "false", "null", "{}", "[]"] {
            let v = format!(r#""{}""#, v);
            let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
            assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
        }
    }

    #[test]
    fn test_string_value_raw_ambiguous() {
        for v in ["true", "false", "null"] {
            let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
            assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
        }
        for v in ["{}", "[]"] {
            let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
            assert_eq!(format_no_color(&rec), format!(r#"k="{}""#, v));
        }
    }

    #[test]
    fn test_string_value_json_number() {
        for v in ["42", "42.42", "-42", "-42.42"] {
            let v = format!(r#""{}""#, v);
            let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
            assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
        }
        for v in [
            "42128731867381927389172983718293789127389172938712983718927",
            "42.128731867381927389172983718293789127389172938712983718927",
        ] {
            let qv = format!(r#""{}""#, v);
            let rec = Record::from_fields(&[("k", EncodedString::json(&qv).into())]);
            assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
        }
    }

    #[test]
    fn test_string_value_raw_number() {
        for v in ["42", "42.42", "-42", "-42.42"] {
            let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
            assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
        }
        for v in [
            "42128731867381927389172983718293789127389172938712983718927",
            "42.128731867381927389172983718293789127389172938712983718927",
        ] {
            let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
            assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
        }
    }

    #[test]
    fn test_string_value_json_version() {
        let v = "1.1.0";
        let qv = format!(r#""{}""#, v);
        let rec = Record::from_fields(&[("k", EncodedString::json(&qv).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }

    #[test]
    fn test_string_value_raw_version() {
        let v = "1.1.0";
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }

    #[test]
    fn test_string_value_json_hyphen() {
        let v = "-";
        let qv = format!(r#""{}""#, v);
        let rec = Record::from_fields(&[("k", EncodedString::json(&qv).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }

    #[test]
    fn test_string_value_raw_hyphen() {
        let v = "-";
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }

    #[test]
    fn test_string_value_trailing_space() {
        let input = "test message\n";
        let golden = r#""test message""#;
        let rec = Record::from_fields(&[("k", EncodedString::raw(&input).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, golden));
    }

    #[test]
    fn test_message_empty() {
        let rec = Record {
            message: Some(EncodedString::raw("").into()),
            ..Default::default()
        };

        let result = format_no_color(&rec);
        assert_eq!(&result, "", "{}", result);
    }

    #[test]
    fn test_message_double_quoted() {
        let rec = Record {
            message: Some(EncodedString::raw(r#""hello, world""#).into()),
            ..Default::default()
        };

        let result = format_no_color(&rec);
        assert_eq!(&result, r#"'"hello, world"'"#, "{}", result);
    }

    #[test]
    fn test_message_single_quoted() {
        let rec = Record {
            message: Some(EncodedString::raw(r#"'hello, world'"#).into()),
            ..Default::default()
        };

        let result = format_no_color(&rec);
        assert_eq!(&result, r#""'hello, world'""#, "{}", result);
    }

    #[test]
    fn test_message_single_and_double_quoted() {
        let rec = Record {
            message: Some(EncodedString::raw(r#"'hello, "world"'"#).into()),
            ..Default::default()
        };

        let result = format_no_color(&rec);
        assert_eq!(&result, r#"`'hello, "world"'`"#, "{}", result);
    }

    #[test]
    fn test_message_control_chars() {
        let rec = Record {
            message: Some(EncodedString::raw("hello, \x1b[33mworld\x1b[0m").into()),
            ..Default::default()
        };

        let result = format_no_color(&rec);
        assert_eq!(&result, r#""hello, \u001b[33mworld\u001b[0m""#, "{}", result);
    }

    #[test]
    fn test_message_spaces_only() {
        let rec = Record {
            message: Some(EncodedString::raw("    ").into()),
            ..Default::default()
        };

        let result = format_no_color(&rec);
        assert_eq!(&result, r#""#, "{}", result);
    }

    #[test]
    fn test_nested_hidden_fields_flatten() {
        let val = json_raw_value(r#"{"b":{"c":{"d":1,"e":2},"f":3}}"#);
        let rec = Record::from_fields(&[("a", RawObject::Json(&val).into())]);
        let mut fields = IncludeExcludeKeyFilter::default();
        let b = fields.entry("a").entry("b");
        b.exclude();
        b.entry("c").entry("d").include();
        let formatter = RecordFormatterBuilder {
            flatten: true,
            theme: Default::default(), // No theme for consistent test output
            fields: Some(fields.into()),
            ..formatter()
        }
        .build();

        assert_eq!(&formatter.format_to_string(&rec), "a.b.c.d=1 ...");
    }

    #[test]
    fn test_nested_hidden_fields_group_unhide() {
        let val = json_raw_value(r#"{"b":{"c":{"d":1,"e":2},"f":3}}"#);
        let rec = Record::from_fields(&[("a", RawObject::Json(&val).into())]);
        let mut fields = IncludeExcludeKeyFilter::default();
        fields.entry("a.b.c").exclude();
        fields.entry("a.b.c.e").include();
        fields.entry("a.b.c").exclude();
        let formatter = RecordFormatterBuilder {
            flatten: true,
            theme: Default::default(), // No theme for consistent test output
            fields: Some(fields.into()),
            ..formatter()
        }
        .build();

        assert_eq!(&formatter.format_to_string(&rec), "a.b.f=3 ...");
    }

    #[test]
    fn test_nested_hidden_fields_no_flatten() {
        let val = json_raw_value(r#"{"b":{"c":{"d":1,"e":2},"f":3}}"#);
        let rec = Record::from_fields(&[("a", RawObject::Json(&val).into())]);
        let mut fields = IncludeExcludeKeyFilter::default();
        let b = fields.entry("a").entry("b");
        b.exclude();
        b.entry("c").entry("d").include();
        let formatter = RecordFormatterBuilder {
            flatten: false,
            theme: Default::default(), // No theme for consistent test output
            fields: Some(fields.into()),
            ..formatter()
        }
        .build();

        assert_eq!(&formatter.format_to_string(&rec), "a={ b={ c={ d=1 ... } ... } }");
    }

    #[test]
    fn test_caller() {
        let rec = Record {
            caller: Caller {
                name: "test_function".into(),
                file: "test_file.rs".into(),
                line: "42".into(),
            },
            ..Default::default()
        };

        let result = format_no_color(&rec);
        assert_eq!(&result, " @ test_function :: test_file.rs:42", "{}", result);
    }

    #[test]
    fn test_no_op_record_with_source_formatter() {
        let formatter = NoOpRecordWithSourceFormatter;
        let rec = Record::default();
        let rec = rec.with_source(b"src");
        formatter.format_record(&mut Buf::default(), rec);
    }

    #[test]
    fn test_delimited_message_with_colors() {
        let formatter = RecordFormatter {
            message_format: new_message_format(MessageFormat::Delimited, "::"),
            ..formatter().build()
        };

        let rec = Record {
            ts: Some(Timestamp::new("2000-01-02T03:04:05.123Z")),
            fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
            ..Default::default()
        };
        assert_eq!(
            formatter.format_to_string(&rec),
            "\u{1b}[0;2;3m00-01-02 03:04:05.123\u{1b}[0m \u{1b}[0;2;3m:: \u{1b}[0;32ma\u{1b}[0;2m=\u{1b}[0;94m42\u{1b}[0m"
        );
    }

    #[test]
    fn test_auto_quoted_message() {
        let formatter = RecordFormatter {
            message_format: new_message_format(MessageFormat::AutoQuoted, ""),
            theme: Default::default(),
            ..formatter().build()
        };

        let mut rec = Record {
            message: Some(EncodedString::raw("m").into()),
            fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
            ..Default::default()
        };
        assert_eq!(formatter.format_to_string(&rec), "m a=42");

        rec.fields = Default::default();
        assert_eq!(formatter.format_to_string(&rec), "m");

        rec.message = Some(EncodedString::raw("m x=1").into());
        assert_eq!(formatter.format_to_string(&rec), r#""m x=1""#);

        rec.message = Some(EncodedString::raw("m '1'").into());
        assert_eq!(formatter.format_to_string(&rec), r#"m '1'"#);

        rec.message = Some(EncodedString::raw(r#"m '1' and "2""#).into());
        assert_eq!(formatter.format_to_string(&rec), r#"m '1' and "2""#);

        rec.message = Some(EncodedString::raw(r#"m x='1' and y="2""#).into());
        assert_eq!(formatter.format_to_string(&rec), r#"`m x='1' and y="2"`"#);

        rec.message = Some(EncodedString::raw("'m' `1`").into());
        assert_eq!(formatter.format_to_string(&rec), r#""'m' `1`""#);

        rec.message = Some(EncodedString::raw("").into());
        assert_eq!(formatter.format_to_string(&rec), r#""#);

        rec.ts = Some(Timestamp::new("2000-01-02T03:04:05.123Z"));
        assert_eq!(formatter.format_to_string(&rec), r#"00-01-02 03:04:05.123"#);
    }

    #[test]
    fn test_always_quoted_message() {
        let formatter = RecordFormatter {
            message_format: new_message_format(MessageFormat::AlwaysQuoted, ""),
            theme: Default::default(),
            ..formatter().build()
        };

        let mut rec = Record {
            message: Some(EncodedString::raw("m").into()),
            fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
            ..Default::default()
        };
        assert_eq!(formatter.format_to_string(&rec), r#""m" a=42"#);

        rec.message = Some(EncodedString::raw("m x='1'").into());
        assert_eq!(formatter.format_to_string(&rec), r#""m x='1'" a=42"#);

        rec.message = Some(EncodedString::raw(r#""m" x='1'"#).into());
        assert_eq!(formatter.format_to_string(&rec), r#"`"m" x='1'` a=42"#);

        rec.message = Some(EncodedString::raw(r#"m x="1""#).into());
        assert_eq!(formatter.format_to_string(&rec), r#"'m x="1"' a=42"#);

        rec.message = Some(EncodedString::raw(r#"m `x`="1"|'2'"#).into());
        assert_eq!(formatter.format_to_string(&rec), r#""m `x`=\"1\"|'2'" a=42"#);

        rec.fields = Default::default();
        rec.message = Some(EncodedString::raw("m").into());
        assert_eq!(formatter.format_to_string(&rec), r#""m""#);
    }

    #[test]
    fn test_always_double_quoted_message() {
        let formatter = RecordFormatter {
            message_format: new_message_format(MessageFormat::AlwaysDoubleQuoted, ""),
            theme: Default::default(),
            ..formatter().build()
        };

        let mut rec = Record {
            message: Some(EncodedString::raw("m").into()),
            fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
            ..Default::default()
        };
        assert_eq!(formatter.format_to_string(&rec), r#""m" a=42"#);

        rec.fields = Default::default();
        assert_eq!(formatter.format_to_string(&rec), r#""m""#);
    }

    #[test]
    fn test_raw_message() {
        let formatter = RecordFormatter {
            message_format: new_message_format(MessageFormat::Raw, ""),
            theme: Default::default(),
            ..formatter().build()
        };

        let mut rec = Record {
            message: Some(EncodedString::raw("m 1").into()),
            fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
            ..Default::default()
        };
        assert_eq!(formatter.format_to_string(&rec), r#"m 1 a=42"#);

        rec.fields = Default::default();
        assert_eq!(formatter.format_to_string(&rec), r#"m 1"#);
    }

    #[test]
    fn test_delimited_message() {
        let formatter = RecordFormatter {
            message_format: new_message_format(MessageFormat::Delimited, "::"),
            theme: Default::default(),
            ..formatter().build()
        };

        let mut rec = Record {
            message: Some(EncodedString::raw("'message' 1").into()),
            fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
            ..Default::default()
        };
        assert_eq!(formatter.format_to_string(&rec), r#""'message' 1" :: a=42"#);

        rec.message = Some(EncodedString::raw(r#"`'message' "1"`"#).into());
        assert_eq!(formatter.format_to_string(&rec), r#""`'message' \"1\"`" :: a=42"#);

        rec.fields = Default::default();
        assert_eq!(formatter.format_to_string(&rec), r#""`'message' \"1\"`""#);

        rec.message = Some(EncodedString::raw("'message' 1").into());
        assert_eq!(formatter.format_to_string(&rec), r#""'message' 1""#);

        rec.message = Some(EncodedString::raw(r#""message" 1"#).into());
        assert_eq!(formatter.format_to_string(&rec), r#"'"message" 1'"#);

        rec.message = Some(EncodedString::raw(r#""message" '1'"#).into());
        assert_eq!(formatter.format_to_string(&rec), r#"`"message" '1'`"#);
    }

    #[test]
    fn test_ascii_mode() {
        // Use record and formatting from testing module
        let (rec, formatting) = crate::testing::ascii::record();

        // Create formatters with each ASCII mode but no theme (for no-color output)
        let formatter_ascii = RecordFormatterBuilder::new()
            .with_timestamp_formatter(
                DateTimeFormatter::new(
                    LinuxDateFormat::new("%b %d %T.%3N").compile(),
                    Tz::FixedOffset(Utc.fix()),
                )
                .into(),
            )
            .with_options(formatting.clone())
            .with_ascii(AsciiMode::On)
            .build();

        let formatter_utf8 = RecordFormatterBuilder::new()
            .with_timestamp_formatter(
                DateTimeFormatter::new(
                    LinuxDateFormat::new("%b %d %T.%3N").compile(),
                    Tz::FixedOffset(Utc.fix()),
                )
                .into(),
            )
            .with_options(formatting)
            .with_ascii(AsciiMode::Off)
            .build();

        // Get formatted output from both formatters (already without ANSI codes)
        let ascii_result = formatter_ascii.format_to_string(&rec);
        let utf8_result = formatter_utf8.format_to_string(&rec);

        // Verify ASCII mode uses ASCII arrow
        assert!(ascii_result.contains("-> "), "ASCII mode should use ASCII arrow");
        // Also verify that it doesn't contain the Unicode arrow
        assert!(!ascii_result.contains("→ "), "ASCII mode should not use Unicode arrow");

        // The ASCII and UTF-8 outputs should be different
        assert_ne!(ascii_result, utf8_result);

        // UTF-8 mode should use Unicode arrow
        assert!(utf8_result.contains("→ "), "UTF-8 mode should use Unicode arrow");
        // Also verify that it doesn't contain the ASCII arrow
        assert!(!utf8_result.contains("-> "), "UTF-8 mode should not use ASCII arrow");
    }

    #[test]
    fn test_punctuation_with_ascii_mode() {
        // Use record and formatting from testing module
        let (_, formatting) = crate::testing::ascii::record();

        // Create formatters with different ASCII modes but no theme
        let ascii_formatter = RecordFormatterBuilder::new()
            .with_timestamp_formatter(
                DateTimeFormatter::new(
                    LinuxDateFormat::new("%y-%m-%d %T.%3N").compile(),
                    Tz::FixedOffset(Utc.fix()),
                )
                .into(),
            )
            .with_options(formatting.clone())
            .with_ascii(AsciiMode::On)
            .build();

        let utf8_formatter = RecordFormatterBuilder::new()
            .with_timestamp_formatter(
                DateTimeFormatter::new(
                    LinuxDateFormat::new("%y-%m-%d %T.%3N").compile(),
                    Tz::FixedOffset(Utc.fix()),
                )
                .into(),
            )
            .with_options(formatting)
            .with_ascii(AsciiMode::Off)
            .build();

        // Use test record with source location for testing source_location_separator
        let rec = crate::testing::record_with_source();

        // Format the record with both formatters
        let ascii_result = ascii_formatter.format_to_string(&rec);
        let utf8_result = utf8_formatter.format_to_string(&rec);

        // ASCII result should contain the ASCII arrow
        assert!(ascii_result.contains("-> "), "ASCII result missing expected arrow");

        // UTF-8 result should contain the Unicode arrow
        assert!(utf8_result.contains("→ "), "UTF-8 result missing expected arrow");

        // The outputs should be different
        assert_ne!(ascii_result, utf8_result);
    }
}
