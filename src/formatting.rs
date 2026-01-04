// std imports
use std::sync::Arc;

// workspace imports
use encstr::EncodedString;

// local imports
use crate::{
    ExactIncludeExcludeKeyFilter, IncludeExcludeKeyFilter,
    datefmt::DateTimeFormatter,
    filtering::IncludeExcludeSetting,
    fmtx::{OptimizedBuf, Push, aligned_left, centered},
    model::{self, Level, RawValue},
    settings::{AsciiMode, Formatting, ResolvedPunctuation},
    theme::{Element, StylingPush, Theme},
};

// ---

/// Result of formatting a field.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatResult {
    /// Field was formatted and shown.
    Formatted,
    /// Field was hidden by user filter (should trigger ellipsis).
    HiddenByUser,
    /// Field was hidden by predefined filter (silent skip, no ellipsis).
    HiddenByPredefined,
}

impl FormatResult {
    #[inline]
    fn is_formatted(self) -> bool {
        self == FormatResult::Formatted
    }

    #[inline]
    fn is_hidden_by_user(self) -> bool {
        self == FormatResult::HiddenByUser
    }
}

// test imports
#[cfg(test)]
use crate::testing::Sample;

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

impl<T: RecordWithSourceFormatter + ?Sized> RecordWithSourceFormatter for &T {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, rec: model::RecordWithSource) {
        (**self).format_record(buf, rec)
    }
}

impl<T: RecordWithSourceFormatter + ?Sized> RecordWithSourceFormatter for Arc<T> {
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

pub type DynRecordWithSourceFormatter = Arc<dyn RecordWithSourceFormatter + Send + Sync>;

// ---

#[derive(Default, Clone)]
pub struct RecordFormatterBuilder {
    theme: Option<Arc<Theme>>,
    raw_fields: bool,
    ts_formatter: Option<DateTimeFormatter>,
    hide_empty_fields: bool,
    flatten: bool,
    ascii: AsciiMode,
    always_show_time: bool,
    always_show_level: bool,
    fields: Option<Arc<IncludeExcludeKeyFilter>>,
    predefined_fields: Option<Arc<ExactIncludeExcludeKeyFilter>>,
    cfg: Option<Formatting>,
    punctuation: Option<Arc<ResolvedPunctuation>>,
    message_format: Option<DynMessageFormat>,
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

    pub fn with_timestamp_formatter(self, value: DateTimeFormatter) -> Self {
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

    pub fn with_predefined_field_filter(self, value: Arc<ExactIncludeExcludeKeyFilter>) -> Self {
        Self {
            predefined_fields: Some(value),
            ..self
        }
    }

    pub fn with_punctuation(self, value: Arc<ResolvedPunctuation>) -> Self {
        Self {
            punctuation: Some(value),
            ..self
        }
    }

    pub fn with_message_format(self, value: DynMessageFormat) -> Self {
        Self {
            message_format: Some(value),
            ..self
        }
    }

    pub fn build(self) -> RecordFormatter {
        let cfg = self.cfg.unwrap_or_default();
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
            predefined_fields: self.predefined_fields.unwrap_or_default(),
            message_format: self
                .message_format
                .unwrap_or_else(|| DynMessageFormat::new(&cfg, self.ascii)),
            punctuation,
        }
    }
}

#[cfg(test)]
impl Sample for RecordFormatterBuilder {
    fn sample() -> Self {
        Self {
            ascii: AsciiMode::On,
            ..Default::default()
        }
    }
}

pub struct RecordFormatter {
    theme: Arc<Theme>,
    unescape_fields: bool,
    ts_formatter: DateTimeFormatter,
    ts_width: usize,
    hide_empty_fields: bool,
    flatten: bool,
    always_show_time: bool,
    always_show_level: bool,
    fields: Arc<IncludeExcludeKeyFilter>,
    predefined_fields: Arc<ExactIncludeExcludeKeyFilter>,
    message_format: DynMessageFormat,
    punctuation: Arc<ResolvedPunctuation>,
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
            let level = level.or(self.always_show_level.then_some(b"(?)"));
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
                    let result =
                        self.format_field(s, k, *v, &mut fs, Some(&self.fields), Some(&self.predefined_fields));
                    some_fields_hidden |= result.is_hidden_by_user();
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
                                if !caller.line.is_empty() {
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
        predefined_filter: Option<&ExactIncludeExcludeKeyFilter>,
    ) -> FormatResult {
        let mut fv = FieldFormatter::new(self);
        fv.format(
            s,
            key,
            value,
            fs,
            filter,
            IncludeExcludeSetting::Unspecified,
            predefined_filter,
            IncludeExcludeSetting::Unspecified,
        )
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
            }
            _ => {
                self.format_field(s, "msg", value, fs, Some(self.fields.as_ref()), None);
            }
        }
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
        buf.extend_from_slice(self.value.as_slices().0);
        buf.extend_from_slice(self.value.as_slices().1);
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

    #[allow(clippy::too_many_arguments)]
    fn format<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        key: &str,
        value: RawValue<'a>,
        fs: &mut FormattingState,
        filter: Option<&IncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
        predefined_filter: Option<&ExactIncludeExcludeKeyFilter>,
        predefined_setting: IncludeExcludeSetting,
    ) -> FormatResult {
        let (predefined_filter, predefined_setting, predefined_leaf) = match predefined_filter {
            Some(filter) => {
                let setting = predefined_setting.apply(filter.setting());
                match filter.get(key) {
                    Some(filter) => (Some(filter), setting.apply(filter.setting()), filter.leaf()),
                    None => (None, setting, true),
                }
            }
            None => (None, predefined_setting, true),
        };
        if predefined_setting == IncludeExcludeSetting::Exclude && predefined_leaf {
            return FormatResult::HiddenByPredefined;
        }

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
            return FormatResult::HiddenByUser;
        }

        let has_predefined_filter = predefined_filter.is_some();
        let rollback_pos =
            if (self.rf.hide_empty_fields || has_predefined_filter) && matches!(value, RawValue::Object(_)) {
                Some(s.batch(|buf| buf.len()))
            } else {
                None
            };

        let ffv = self.begin(s, key, value, fs);
        let has_content = if self.rf.unescape_fields {
            self.format_value(s, value, fs, filter, predefined_filter, setting, predefined_setting)
        } else {
            s.element(Element::String, |s| {
                s.batch(|buf| buf.extend(value.raw_str().as_bytes()))
            });
            true
        };

        self.end(fs, ffv);

        match (rollback_pos, has_content) {
            (Some(pos), false) => {
                s.batch(|buf| buf.truncate(pos));
                FormatResult::HiddenByPredefined
            }
            _ => FormatResult::Formatted,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn format_value<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        value: RawValue<'a>,
        fs: &mut FormattingState,
        filter: Option<&IncludeExcludeKeyFilter>,
        predefined_filter: Option<&ExactIncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
        predefined_setting: IncludeExcludeSetting,
    ) -> bool {
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
                let mut any_fields_formatted = false;
                s.element(Element::Object, |s| {
                    if !fs.flatten {
                        s.batch(|buf| buf.push(b'{'));
                    }
                    let mut some_fields_hidden_by_user = false;
                    for (k, v) in item.fields.iter() {
                        if !self.rf.hide_empty_fields || !v.is_empty() {
                            let result =
                                self.format(s, k, *v, fs, filter, setting, predefined_filter, predefined_setting);
                            any_fields_formatted |= result.is_formatted();
                            some_fields_hidden_by_user |= result.is_hidden_by_user();
                        } else {
                            some_fields_hidden_by_user = true;
                        }
                    }
                    if !fs.flatten {
                        if some_fields_hidden_by_user {
                            s.element(Element::Ellipsis, |s| {
                                s.batch(|buf| buf.extend(self.rf.punctuation.hidden_fields_indicator.as_bytes()))
                            });
                        }
                        s.batch(|buf| {
                            if !item.fields.is_empty() {
                                buf.push(b' ');
                            }
                            buf.push(b'}');
                        });
                    }
                    fs.some_nested_fields_hidden |= some_fields_hidden_by_user;
                });
                return any_fields_formatted;
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
                        self.format_value(
                            s,
                            *v,
                            fs,
                            None,
                            None,
                            IncludeExcludeSetting::Unspecified,
                            IncludeExcludeSetting::Unspecified,
                        );
                    }
                    s.batch(|buf| buf.push(b']'));
                });
            }
        };
        true
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

    pub type DynFormat = Arc<dyn Format + Send + Sync>;

    #[derive(Clone)]
    pub struct DynMessageFormat {
        format: DynFormat,
        pub delimited: bool,
    }

    impl DynMessageFormat {
        pub fn new(formatting: &super::Formatting, ascii: super::AsciiMode) -> Self {
            new_message_format(formatting.message.format, || {
                formatting.punctuation.message_delimiter.resolve(ascii)
            })
        }
    }

    impl std::ops::Deref for DynMessageFormat {
        type Target = DynFormat;

        fn deref(&self) -> &Self::Target {
            &self.format
        }
    }

    pub fn new_message_format<D: DisplayResolve>(setting: MessageFormat, delimiter: D) -> DynMessageFormat {
        let (format, delimited): (DynFormat, _) = match setting {
            MessageFormat::AutoQuoted => (Arc::new(MessageFormatAutoQuoted), false),
            MessageFormat::AlwaysQuoted => (Arc::new(MessageFormatAlwaysQuoted), false),
            MessageFormat::AlwaysDoubleQuoted => (Arc::new(MessageFormatDoubleQuoted), false),
            MessageFormat::Delimited => {
                let delimiter = format!(" {} ", delimiter.resolve());
                let n = delimiter.len();
                (Arc::new(MessageFormatDelimited::new(delimiter).rtrim(n)), true)
            }
            MessageFormat::Raw => (Arc::new(MessageFormatRaw), false),
        };
        DynMessageFormat { format, delimited }
    }

    // ---

    pub trait DisplayResolve {
        type Output: std::fmt::Display;

        fn resolve(self) -> Self::Output;
    }

    impl<'a> DisplayResolve for &'a str {
        type Output = Self;

        fn resolve(self) -> &'a str {
            self
        }
    }

    impl<'a, F> DisplayResolve for F
    where
        F: FnOnce() -> &'a str,
    {
        type Output = &'a str;

        fn resolve(self) -> &'a str {
            self()
        }
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
                    ) && !looks_like_number(&buf[begin..])
                }
            } else {
                false
            };

            if plain {
                return Ok(());
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            if !mask.intersects(Flag::Backtick | Flag::Control) {
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

            if !mask.intersects(Flag::EqualSign | Flag::Control | Flag::NewLine | Flag::Backslash)
                && !matches!(buf[begin..], [b'"', ..] | [b'\'', ..] | [b'`', ..])
            {
                return Ok(());
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(());
            }

            if !mask.intersects(Flag::Backtick | Flag::Control) {
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

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'"');
                return Ok(());
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf[begin] = b'\'';
                buf.push(b'\'');
                return Ok(());
            }

            if !mask.intersects(Flag::Backtick | Flag::Control) {
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

            if !mask.intersects(Flag::Backtick | Flag::Control) {
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
        const CT: Mask = mask!(Flag::Control); // 0x00..0x1F except 0x09, 0x0A, 0x0D
        const DQ: Mask = mask!(Flag::DoubleQuote); // 0x22
        const SQ: Mask = mask!(Flag::SingleQuote); // 0x27
        const BS: Mask = mask!(Flag::Backslash); // 0x5C
        const BT: Mask = mask!(Flag::Backtick); // 0x60
        const SP: Mask = mask!(Flag::Space); // 0x20
        const TB: Mask = mask!(Flag::Tab); // 0x09
        const NL: Mask = mask!(Flag::NewLine); // 0x0A, 0x0D
        const EQ: Mask = mask!(Flag::EqualSign); // 0x3D
        const HY: Mask = mask!(Flag::Minus); // Hyphen, 0x2D
        const DO: Mask = mask!(Flag::Dot); // Dot, 0x2E
        const DD: Mask = mask!(Flag::Digit); // Decimal digit, 0x30..0x39
        const __: Mask = mask!(Flag::Other);
        [
            //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
            CT, CT, CT, CT, CT, CT, CT, CT, CT, TB, NL, CT, CT, NL, CT, CT, // 0
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
        Tab,
        NewLine,
        EqualSign,
        Digit,
        Minus,
        Dot,
        Other,
    }

    type Mask = EnumSet<Flag>;
}

#[cfg(test)]
mod tests;
