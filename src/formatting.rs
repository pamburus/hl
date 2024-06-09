// std imports
use std::{ops::Range, sync::Arc};

// workspace imports
use encstr::EncodedString;

// local imports
use crate::{
    datefmt::{DateTimeFormatter, TextWidth},
    filtering::IncludeExcludeSetting,
    fmtx::{aligned_left, centered, OptimizedBuf, Push},
    model::{self, Caller, Level, RawValue},
    settings::{ExpandOption, FlattenOption, Formatting, Punctuation},
    theme::{Element, StylingPush, Theme},
    IncludeExcludeKeyFilter,
};

// relative imports
use string::{Format, MessageFormatAuto, ValueFormatAuto};

// ---

type Buf = Vec<u8>;

// ---

pub trait RecordWithSourceFormatter {
    fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: model::RecordWithSource);
}

pub struct RawRecordFormatter {}

impl RecordWithSourceFormatter for RawRecordFormatter {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, _prefix_range: Range<usize>, rec: model::RecordWithSource) {
        buf.extend_from_slice(rec.source);
    }
}

impl<T: RecordWithSourceFormatter> RecordWithSourceFormatter for &T {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: model::RecordWithSource) {
        (**self).format_record(buf, prefix_range, rec)
    }
}

impl RecordWithSourceFormatter for Box<dyn RecordWithSourceFormatter> {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: model::RecordWithSource) {
        (**self).format_record(buf, prefix_range, rec)
    }
}

// ---

pub struct RecordFormatterSettings {
    pub theme: Arc<Theme>,
    pub unescape_fields: bool,
    pub ts_formatter: DateTimeFormatter,
    pub hide_empty_fields: bool,
    pub flatten: bool,
    pub expand: ExpandOption,
    pub always_show_time: bool,
    pub always_show_level: bool,
    pub fields: Arc<IncludeExcludeKeyFilter>,
    pub punctuation: Arc<Punctuation>,
}

impl RecordFormatterSettings {
    pub fn with<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        f(&mut self);
        self
    }
}

impl From<Formatting> for RecordFormatterSettings {
    fn from(cfg: Formatting) -> Self {
        Self {
            theme: Arc::new(Theme::default()),
            unescape_fields: true,
            ts_formatter: DateTimeFormatter::default(),
            hide_empty_fields: false,
            flatten: cfg.flatten.map(|x| x == FlattenOption::Always).unwrap_or(false),
            expand: cfg.expand.unwrap_or_default(),
            always_show_time: false,
            always_show_level: false,
            fields: Arc::new(IncludeExcludeKeyFilter::default()),
            punctuation: Arc::new(cfg.punctuation.into()),
        }
    }
}

impl Default for RecordFormatterSettings {
    fn default() -> Self {
        Formatting::default().into()
    }
}

// ---

pub struct RecordFormatter {
    theme: Arc<Theme>,
    unescape_fields: bool,
    ts_formatter: DateTimeFormatter,
    hide_empty_fields: bool,
    flatten: bool,
    expand: ExpandOption,
    always_show_time: bool,
    always_show_level: bool,
    fields: Arc<IncludeExcludeKeyFilter>,
    punctuation: Arc<Punctuation>,
    ts_width: TextWidth,
}

impl RecordFormatter {
    pub fn new(settings: RecordFormatterSettings) -> Self {
        let ts_width = settings.ts_formatter.max_width();
        Self {
            theme: settings.theme,
            unescape_fields: settings.unescape_fields,
            ts_formatter: settings.ts_formatter,
            hide_empty_fields: settings.hide_empty_fields,
            flatten: settings.flatten,
            expand: settings.expand,
            always_show_time: settings.always_show_time,
            always_show_level: settings.always_show_level,
            fields: settings.fields,
            punctuation: settings.punctuation,
            ts_width,
        }
    }

    pub fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: &model::Record) {
        let mut fs = FormattingState {
            flatten: self.flatten && self.unescape_fields,
            expand: self.expand.into(),
            prefix: prefix_range,
            ..Default::default()
        };

        self.theme.apply(buf, &rec.level, |s| {
            //
            // time
            //
            if let Some(ts) = &rec.ts {
                fs.ts_width = self.ts_width.chars;
                fs.add_element(|| {});
                s.element(Element::Time, |s| {
                    s.batch(|buf| {
                        aligned_left(buf, self.ts_width.bytes, b' ', |mut buf| {
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
                fs.ts_width = self.ts_width.chars;
                fs.add_element(|| {});
                s.element(Element::Time, |s| {
                    s.batch(|buf| {
                        centered(buf, self.ts_width.bytes, b'-', |mut buf| {
                            buf.extend_from_slice(b"-");
                        });
                    })
                });
            }

            //
            // level
            //
            let level = match rec.level {
                Some(Level::Debug) => Some(b"DBG"),
                Some(Level::Info) => Some(b"INF"),
                Some(Level::Warning) => Some(b"WRN"),
                Some(Level::Error) => Some(b"ERR"),
                _ => None,
            };
            let level = level.or_else(|| self.always_show_level.then(|| b"(?)"));
            if let Some(level) = level {
                fs.has_level = true;
                self.format_level(s, &mut fs, level);
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

            fs.expand = Some(
                fs.expand
                    .unwrap_or_else(|| self.complexity(rec, Some(&self.fields)) >= 128),
            );

            //
            // caller in expanded mode
            //
            let mut caller_formatted = false;
            if fs.expand.unwrap_or(false) {
                if let Some(caller) = &rec.caller {
                    self.format_caller(s, caller);
                    caller_formatted = true;
                };
            }
            //
            // fields
            //
            if fs.format_message_as_field {
                if let Some(value) = &rec.message {
                    self.format_field(s, "msg", *value, &mut fs, Some(self.fields.as_ref()));
                }
            }
            let mut some_fields_hidden = false;
            for (k, v) in rec.fields() {
                if !self.hide_empty_fields || !v.is_empty() {
                    some_fields_hidden |= !self.format_field(s, k, *v, &mut fs, Some(&self.fields));
                }
            }
            if some_fields_hidden || fs.some_fields_hidden {
                if fs.expand == Some(true) {
                    self.expand(s, &mut fs);
                }
                s.element(Element::Ellipsis, |s| {
                    s.batch(|buf| buf.extend_from_slice(self.punctuation.hidden_fields_indicator.as_bytes()))
                });
            }
            //
            // caller in non-expanded mode
            //
            if !caller_formatted {
                if let Some(caller) = &rec.caller {
                    self.format_caller(s, caller);
                };
            }
        });
    }

    #[inline]
    fn complexity(&self, rec: &model::Record, filter: Option<&IncludeExcludeKeyFilter>) -> usize {
        let mut result = 0;
        result += rec.message.map(|x| x.raw_str().len() / 8).unwrap_or(0);
        result += rec.predefined.len();
        result += rec.logger.map(|x| x.len() / 2).unwrap_or(0);
        for (key, value) in rec.fields() {
            if value.is_empty() {
                if self.hide_empty_fields {
                    continue;
                }
                result += 4;
            }

            let setting = IncludeExcludeSetting::Unspecified;
            let (_, setting, leaf) = match filter {
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
                continue;
            }

            result += key.len();
            result += if matches!(value, RawValue::Object(_)) {
                4 + value.raw_str().len() / 2
            } else {
                2 + value.raw_str().len() / 4
            };
        }
        result
    }

    #[inline]
    fn format_caller(&self, s: &mut crate::theme::Styler<Vec<u8>>, caller: &Caller) {
        s.element(Element::Caller, |s| {
            s.batch(|buf| {
                buf.push(b' ');
                buf.extend_from_slice(self.punctuation.source_location_separator.as_bytes())
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
                    if value.source().len() > 192 {
                        fs.expand = Some(true);
                    }
                    fs.add_element(|| {
                        s.reset();
                        s.space();
                    });
                    let prefix = if fs.expand.unwrap_or(true) {
                        Some(|buf: &mut Vec<u8>| {
                            buf.extend(b"...");
                            None
                        })
                    } else {
                        None
                    };
                    let result = s.element(Element::Message, |s| {
                        s.batch(|buf| MessageFormatAuto::new(value, prefix).format(buf))
                    });
                    if result.is_err() {
                        fs.format_message_as_field = true;
                        fs.expand = Some(true);
                    }
                }
            }
            _ => {
                fs.format_message_as_field = true;
            }
        };
    }

    #[inline]
    fn format_level<S: StylingPush<Buf>>(&self, s: &mut S, fs: &mut FormattingState, level: &[u8]) {
        fs.add_element(|| s.space());
        s.element(Element::Level, |s| {
            s.batch(|buf| {
                buf.extend_from_slice(self.punctuation.level_left_separator.as_bytes());
            });
            s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(level)));
            s.batch(|buf| buf.extend_from_slice(self.punctuation.level_right_separator.as_bytes()));
        });
    }

    fn expand<S: StylingPush<Buf>>(&self, s: &mut S, fs: &mut FormattingState) {
        let mut begin = 0;

        s.reset();
        s.batch(|buf| {
            buf.push(b'\n');
            begin = buf.len();
            buf.extend_from_within(fs.prefix.clone());
        });

        fs.dirty = false;
        if fs.ts_width != 0 {
            fs.add_element(|| {});
            s.element(Element::Time, |s| {
                s.batch(|buf| {
                    aligned_left(buf, fs.ts_width, b' ', |_| {});
                })
            });
        }

        if fs.has_level {
            self.format_level(s, fs, b" - ");
            s.reset();
        }

        fs.add_element(|| s.space());
        s.batch(|buf| {
            for _ in 0..fs.depth + 1 {
                buf.extend_from_slice(b"  ");
            }
        });

        // TODO: remove such hacks and replace with direct access to the buffer
        s.batch(|buf| {
            let xl = begin..buf.len();
            fs.expansion_prefix = Some(xl.clone());
        });

        s.element(Element::Bullet, |s| {
            s.batch(|buf| {
                buf.extend_from_slice(b">");
            });
        });
    }
}

impl RecordWithSourceFormatter for RecordFormatter {
    #[inline]
    fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: model::RecordWithSource) {
        RecordFormatter::format_record(self, buf, prefix_range, rec.record)
    }
}

impl From<RecordFormatterSettings> for RecordFormatter {
    #[inline]
    fn from(settings: RecordFormatterSettings) -> Self {
        RecordFormatter::new(settings)
    }
}

// ---

#[derive(Default)]
struct FormattingState {
    flatten: bool,
    expand: Option<bool>,
    prefix: Range<usize>,
    expansion_prefix: Option<Range<usize>>,
    key_prefix: KeyPrefix,
    dirty: bool,
    ts_width: usize,
    has_level: bool,
    depth: usize,
    some_fields_hidden: bool,
    format_message_as_field: bool,
}

impl FormattingState {
    fn add_element(&mut self, add_space: impl FnOnce()) {
        if !self.dirty {
            self.dirty = true;
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
        buf.extend_from_slice(&self.value.head);
        buf.extend_from_slice(&self.value.tail);
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
                let prefix = if fs.expand.unwrap_or(false) {
                    Some(|buf: &mut Vec<u8>| {
                        buf.extend(self.rf.theme.expanded_value_suffix.value.as_bytes());
                        buf.push(b'\n');
                        let prefix = fs.expansion_prefix.clone().unwrap_or(fs.prefix.clone());
                        let l0 = buf.len();
                        buf.extend_from_within(prefix);
                        buf.extend(self.rf.theme.expanded_value_prefix.value.as_bytes());
                        buf.len() - l0
                    })
                } else {
                    None
                };
                s.element(Element::String, |s| {
                    s.batch(|buf| ValueFormatAuto::new(value, prefix).format(buf).unwrap())
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
                if !fs.flatten && (!fs.expand.unwrap_or(false) || value.is_empty()) {
                    s.element(Element::Object, |s| {
                        s.batch(|buf| buf.push(b'{'));
                    });
                }
                let mut some_fields_hidden = false;
                for (k, v) in item.fields.iter() {
                    if !self.rf.hide_empty_fields || !v.is_empty() {
                        some_fields_hidden |= !self.format(s, k, *v, fs, filter, setting);
                    }
                }
                if some_fields_hidden {
                    if !fs.flatten {
                        if fs.expand.unwrap_or(false) {
                            self.rf.expand(s, fs);
                        }
                        s.element(Element::Ellipsis, |s| {
                            s.batch(|buf| buf.extend(self.rf.punctuation.hidden_fields_indicator.as_bytes()))
                        });
                    } else {
                        fs.some_fields_hidden = true;
                    }
                }
                if !fs.flatten && (!fs.expand.unwrap_or(false) || value.is_empty()) {
                    s.element(Element::Object, |s| {
                        s.batch(|buf| {
                            if item.fields.len() != 0 {
                                buf.push(b' ');
                            }
                            buf.push(b'}');
                        });
                    });
                }
            }
            RawValue::Array(value) => {
                let xb = fs.expand.replace(false);
                let item = value.parse::<32>().unwrap();
                s.element(Element::Array, |s| {
                    s.batch(|buf| buf.push(b'['));
                });
                let mut first = true;
                for v in item.iter() {
                    if !first {
                        s.batch(|buf| buf.extend(self.rf.punctuation.array_separator.as_bytes()));
                    } else {
                        first = false;
                    }
                    self.format_value(s, *v, fs, None, IncludeExcludeSetting::Unspecified);
                }
                s.element(Element::Array, |s| {
                    s.batch(|buf| buf.push(b']'));
                });
                fs.expand = xb;
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

        let variant = FormattedFieldVariant::Normal { flatten: fs.flatten };

        if fs.expand.unwrap_or(false) {
            self.rf.expand(s, fs);
        }
        fs.depth += 1;

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

        let sep = if fs.expand.unwrap_or(false) && matches!(value, RawValue::Object(o) if !o.is_empty()) {
            b":"
        } else {
            self.rf.punctuation.field_key_value_separator.as_bytes()
        };
        s.element(Element::Field, |s| {
            s.batch(|buf| buf.extend(sep));
        });

        variant
    }

    #[inline]
    fn end(&mut self, fs: &mut FormattingState, v: FormattedFieldVariant) {
        match v {
            FormattedFieldVariant::Normal { flatten } => {
                fs.depth -= 1;
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
    // third-party imports
    use enumset::{enum_set as mask, EnumSet, EnumSetType};
    use thiserror::Error;

    // workspace imports
    use encstr::{AnyEncodedString, JsonAppender};
    use enumset_ext::EnumSetExt;
    use mline::prefix_lines_within;

    // local imports
    use crate::{
        formatting::WithAutoTrim,
        model::{looks_like_number, MAX_NUMBER_LEN},
    };

    // ---

    /// Error is an error which may occur in the application.
    #[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Error {
        #[error(transparent)]
        ParseError(#[from] encstr::Error),
        #[error("truncated multiline message")]
        TruncatedMultilineMessage,
    }

    // ---

    type Result<T> = std::result::Result<T, Error>;

    // ---

    pub trait Format {
        fn format(&self, buf: &mut Vec<u8>) -> Result<()>;
    }

    // ---

    pub struct ValueFormatAuto<S, P> {
        string: S,
        prefix: Option<P>,
    }

    impl<S, P> ValueFormatAuto<S, P> {
        #[inline]
        pub fn new(string: S, prefix: Option<P>) -> Self {
            Self { string, prefix }
        }
    }

    impl<'a, S, P> Format for ValueFormatAuto<S, P>
    where
        S: AnyEncodedString<'a> + Clone + Copy,
        P: Fn(&mut Vec<u8>) -> usize,
    {
        #[inline]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            if self.string.is_empty() {
                buf.extend(r#""""#.as_bytes());
                return Ok(());
            }

            let begin = buf.len();
            buf.with_auto_trim(|buf| ValueFormatRaw::new(self.string).format(buf))?;

            let mut mask = Mask::EMPTY;

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

            const Z: Mask = Mask::EMPTY;
            const XS: Mask = mask!(Flag::Control | Flag::ExtendedSpace);
            const BTXS: Mask = mask!(Flag::Control | Flag::ExtendedSpace | Flag::Backtick);

            match (mask & BTXS, &self.prefix) {
                (Z, _) | (XS, None) => {
                    buf.push(b'`');
                    buf.push(b'`');
                    buf[begin..].rotate_right(1);
                    Ok(())
                }
                (XS | BTXS, Some(prefix)) => {
                    let l0 = buf.len();
                    let pl = prefix(buf);
                    let n = buf.len() - l0;
                    buf[begin..].rotate_right(n);
                    prefix_lines_within(buf, begin + n.., 1.., (begin + n - pl)..(begin + n));
                    Ok(())
                }
                _ => {
                    buf.truncate(begin);
                    ValueFormatDoubleQuoted::new(self.string).format(buf)
                }
            }
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
            Ok(self.string.decode(buf)?)
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
            self.string.format_json(buf)
        }
    }

    // ---

    pub struct MessageFormatAuto<S, P> {
        string: S,
        prefix: Option<P>,
    }

    impl<S, P> MessageFormatAuto<S, P> {
        #[inline(always)]
        pub fn new(string: S, prefix: Option<P>) -> Self {
            Self { string, prefix }
        }
    }

    impl<'a, S, P> Format for MessageFormatAuto<S, P>
    where
        S: AnyEncodedString<'a> + Clone + Copy,
        P: Fn(&mut Vec<u8>) -> Option<usize>,
    {
        #[inline(always)]
        fn format(&self, buf: &mut Vec<u8>) -> Result<()> {
            if self.string.is_empty() {
                return Ok(());
            }

            let begin = buf.len();
            buf.with_auto_trim(|buf| MessageFormatRaw::new(self.string).format(buf))?;

            let mut mask = Mask::EMPTY;

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

            const Z: Mask = Mask::EMPTY;
            const XS: Mask = mask!(Flag::Control | Flag::ExtendedSpace);
            const BTXS: Mask = mask!(Flag::Control | Flag::ExtendedSpace | Flag::Backtick);

            match (mask & BTXS, &self.prefix) {
                (Z, _) | (XS, None) => {
                    buf.push(b'`');
                    buf.push(b'`');
                    buf[begin..].rotate_right(1);
                    Ok(())
                }
                (XS | BTXS, Some(prefix)) => {
                    let l0 = buf.len();
                    let pl = prefix(buf);
                    let n = buf.len() - l0;
                    if let Some(pl) = pl {
                        buf[begin..].rotate_right(n);
                        prefix_lines_within(buf, begin + n.., 1.., (begin + n - pl)..(begin + n));
                        Ok(())
                    } else {
                        buf.copy_within(l0.., begin);
                        buf.truncate(begin + n);
                        Err(Error::TruncatedMultilineMessage)
                    }
                }
                _ => {
                    buf.truncate(begin);
                    MessageFormatDoubleQuoted::new(self.string).format(buf)
                }
            }
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
            Ok(self.string.decode(buf)?)
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
            self.string.format_json(buf)
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
    use super::*;
    use crate::{
        datefmt::LinuxDateFormat,
        model::{RawObject, Record, RecordFields},
        settings::Punctuation,
        theme::Theme,
        themecfg::testing,
        timestamp::Timestamp,
        timezone::Tz,
    };
    use chrono::{Offset, Utc};
    use encstr::EncodedString;
    use model::RecordWithSourceConstructor;
    use serde_json as json;

    trait FormatToVec<R> {
        fn format_to_vec(&self, rec: R) -> Vec<u8>;
    }

    trait FormatToString<R> {
        fn format_to_string(&self, rec: R) -> String;
    }

    impl<'a> FormatToVec<&'a Record<'a>> for RecordFormatter {
        fn format_to_vec(&self, rec: &'a Record<'a>) -> Vec<u8> {
            let mut buf = Vec::new();
            self.format_record(&mut buf, 0..0, rec);
            buf
        }
    }

    impl<'a> FormatToString<&'a Record<'a>> for RecordFormatter {
        fn format_to_string(&self, rec: &'a Record<'a>) -> String {
            String::from_utf8(self.format_to_vec(rec)).unwrap()
        }
    }

    impl<'a, T> FormatToVec<model::RecordWithSource<'a>> for T
    where
        T: RecordWithSourceFormatter,
    {
        fn format_to_vec(&self, rec: model::RecordWithSource<'a>) -> Vec<u8> {
            let mut buf = Vec::new();
            (&self).format_record(&mut buf, 0..0, rec);
            buf
        }
    }

    impl<'a, T> FormatToString<model::RecordWithSource<'a>> for T
    where
        T: RecordWithSourceFormatter,
    {
        fn format_to_string(&self, rec: model::RecordWithSource<'a>) -> String {
            String::from_utf8(self.format_to_vec(rec)).unwrap()
        }
    }

    fn settings() -> RecordFormatterSettings {
        RecordFormatterSettings {
            theme: Arc::new(Theme::from(testing::theme().unwrap())),
            ts_formatter: DateTimeFormatter::new(
                LinuxDateFormat::new("%y-%m-%d %T.%3N").compile(),
                Tz::FixedOffset(Utc.fix()),
            ),
            always_show_time: false,
            punctuation: Arc::new(Punctuation {
                logger_name_separator: ":".into(),
                field_key_value_separator: "=".into(),
                string_opening_quote: "'".into(),
                string_closing_quote: "'".into(),
                source_location_separator: "@ ".into(),
                hidden_fields_indicator: " ...".into(),
                level_left_separator: "|".into(),
                level_right_separator: "|".into(),
                input_number_prefix: "#".into(),
                input_number_left_separator: "".into(),
                input_number_right_separator: " | ".into(),
                input_name_left_separator: "".into(),
                input_name_right_separator: " | ".into(),
                input_name_clipping: "...".into(),
                input_name_common_part: "...".into(),
                array_separator: ",".into(),
            }),
            ..Default::default()
        }
    }

    fn formatter() -> RecordFormatter {
        settings().into()
    }

    fn format(rec: &Record) -> String {
        formatter().format_to_string(rec)
    }

    fn format_no_color(rec: &Record) -> String {
        let formatter = RecordFormatter::new(settings().with(|s| s.theme = Default::default()));
        formatter.format_to_string(rec)
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
                fields: RecordFields {
                    head: heapless::Vec::from_slice(fields).unwrap(),
                    ..Default::default()
                },
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
            caller: Some(Caller::Text("tc")),
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[("k_a", RawValue::from(RawObject::Json(&ka)))]).unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            format(&rec),
            "\u{1b}[0;2;3m00-01-02 03:04:05.123 \u{1b}[0;36m|\u{1b}[0;95mDBG\u{1b}[0;36m|\u{1b}[0;2;3m \u{1b}[0;2;4mtl:\u{1b}[0m \u{1b}[0;1;39mtm \u{1b}[0;32mk-a\u{1b}[0;2m=\u{1b}[0;33m{ \u{1b}[0;32mva\u{1b}[0;2m=\u{1b}[0;33m{ \u{1b}[0;32mkb\u{1b}[0;2m=\u{1b}[0;94m42 \u{1b}[0;32mkc\u{1b}[0;2m=\u{1b}[0;94m43\u{1b}[0;33m } }\u{1b}[0;2;3m @ tc\u{1b}[0m",
        );

        let formatter = RecordFormatter::new(settings().with(|s| s.flatten = true));
        assert_eq!(
            formatter.format_to_string(&rec),
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
    fn test_timestamp_none_always_show() {
        let rec = Record {
            message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
            ..Default::default()
        };

        let formatter = RecordFormatter::new(settings().with(|s| s.always_show_time = true));
        assert_eq!(
            formatter.format_to_string(&rec),
            "\u{1b}[0;2;3m---------------------\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m",
        );
    }

    #[test]
    fn test_level_none() {
        let rec = Record {
            message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
            ..Default::default()
        };

        assert_eq!(format(&rec), "\u{1b}[0;1;39mtm\u{1b}[0m",);
    }

    #[test]
    fn test_level_none_always_show() {
        let rec = Record {
            message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
            ..Default::default()
        };

        let formatter = RecordFormatter::new(settings().with(|s| s.always_show_level = true));
        assert_eq!(
            formatter.format_to_string(&rec),
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
    fn test_expand_with_hidden() {
        let formatter = RecordFormatter::new(settings().with(|s| {
            let mut fields = IncludeExcludeKeyFilter::default();
            fields.entry("b").exclude();
            s.theme = Default::default();
            s.expand = ExpandOption::Always;
            s.fields = fields.into();
        }));

        let source = b"m a=1 b=2 c=3";
        let rec = Record {
            message: Some(EncodedString::raw("m").into()),
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[
                    ("a", EncodedString::raw("1").into()),
                    ("b", EncodedString::raw("2").into()),
                    ("c", EncodedString::raw("3").into()),
                ])
                .unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = formatter.format_to_string(rec.with_source(source));
        assert_eq!(&result, "m\n  > a=1\n  > c=3\n  > ...", "{}", result);
    }

    #[test]
    fn test_caller_file_line() {
        let format = |file, line| {
            let rec = Record {
                message: Some(EncodedString::raw("m").into()),
                caller: Some(Caller::FileLine(file, line)),
                ..Default::default()
            };

            format_no_color(&rec)
        };

        assert_eq!(format("f", "42"), r#"m @ f:42"#);
        assert_eq!(format("f", ""), r#"m @ f"#);
        assert_eq!(format("", "42"), r#"m @ :42"#);
    }
}
