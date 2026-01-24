// std imports
use std::{
    mem::{replace, take},
    ops::{Deref, DerefMut, Range},
    sync::Arc,
};

// third-party imports
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone};
use enumset::{EnumSet, EnumSetType};
use itertools::izip;

// workspace imports
use encstr::EncodedString;

// local imports
use crate::{
    ExactIncludeExcludeKeyFilter, IncludeExcludeKeyFilter,
    datefmt::{DateTimeFormatter, TextWidth},
    filtering::IncludeExcludeSetting,
    fmtx::{OptimizedBuf, Push, aligned_left},
    model::{self, Caller, Level, RawValue},
    scanning::{Delimit, Newline, SearchExt},
    settings::{self, AsciiMode, ExpansionMode, Formatting, ResolvedPunctuation},
    syntax::*,
    theme::{Element, Styler, StylingPush, Theme},
};

// test imports
#[cfg(test)]
use crate::testing::Sample;

// relative imports
use string::{DynMessageFormat, ExtendedSpaceAction, Format, ValueFormatAuto};

// ---

type Buf = Vec<u8>;

// ---

#[derive(Clone, Debug, Default)]
pub struct Expansion {
    pub mode: ExpansionMode,
}

impl Expansion {
    #[inline(always)]
    pub fn with_mode(mut self, mode: ExpansionMode) -> Self {
        self.mode = mode;
        self
    }

    #[inline(always)]
    pub fn profile(&self) -> &ExpansionProfile {
        match self.mode {
            ExpansionMode::Never => &ExpansionProfile::NEVER,
            ExpansionMode::Inline => &ExpansionProfile::INLINE,
            ExpansionMode::Auto => &ExpansionProfile::AUTO,
            ExpansionMode::Always => &ExpansionProfile::ALWAYS,
        }
    }
}

impl From<settings::ExpansionOptions> for Expansion {
    #[inline(always)]
    fn from(options: settings::ExpansionOptions) -> Self {
        Self {
            mode: options.mode.unwrap_or_default(),
        }
    }
}

impl From<settings::ExpansionMode> for Expansion {
    #[inline(always)]
    fn from(mode: settings::ExpansionMode) -> Self {
        Self { mode }
    }
}

// ---

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MultilineExpansion {
    #[default]
    Standard,
    Disabled,
    Inline,
}

// ---

#[derive(Clone, Debug)]
pub struct ExpansionThresholds {}

// ---

#[derive(Clone, Debug)]
pub struct ExpansionProfile {
    pub multiline: MultilineExpansion,
    pub thresholds: ExpansionThresholds,
    pub expand_all: bool,
}

impl ExpansionProfile {
    pub const NEVER: Self = Self {
        multiline: MultilineExpansion::Disabled,
        thresholds: ExpansionThresholds {},
        expand_all: false,
    };

    pub const ALWAYS: Self = Self {
        multiline: MultilineExpansion::Standard,
        thresholds: ExpansionThresholds {},
        expand_all: true,
    };

    pub const INLINE: Self = Self {
        multiline: MultilineExpansion::Inline,
        thresholds: ExpansionThresholds {},
        expand_all: false,
    };

    pub const AUTO: Self = Self {
        multiline: MultilineExpansion::Standard,
        thresholds: ExpansionThresholds {},
        expand_all: false,
    };
}

impl Default for &ExpansionProfile {
    fn default() -> Self {
        &ExpansionProfile::NEVER
    }
}

// ---

pub trait RecordWithSourceFormatter {
    fn format_record(&self, buf: &mut Buf, prefix: Range<usize>, rec: model::RecordWithSource);
}

pub struct RawRecordFormatter {}

impl RecordWithSourceFormatter for RawRecordFormatter {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix: Range<usize>, rec: model::RecordWithSource) {
        let mut first = true;
        for line in Newline.into_searcher().split(rec.source) {
            if !first {
                buf.push(b'\n');
                buf.extend_from_within(prefix.clone());
            }
            first = false;
            buf.extend_from_slice(line);
        }
    }
}

impl<T: RecordWithSourceFormatter + ?Sized> RecordWithSourceFormatter for &T {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix: Range<usize>, rec: model::RecordWithSource) {
        (**self).format_record(buf, prefix, rec)
    }
}

impl<T: RecordWithSourceFormatter + ?Sized> RecordWithSourceFormatter for Arc<T> {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix: Range<usize>, rec: model::RecordWithSource) {
        (**self).format_record(buf, prefix, rec)
    }
}

// ---

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoOpRecordWithSourceFormatter;

impl RecordWithSourceFormatter for NoOpRecordWithSourceFormatter {
    #[inline(always)]
    fn format_record(&self, _: &mut Buf, _: Range<usize>, _: model::RecordWithSource) {}
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
    expansion: Option<Expansion>,
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

    pub fn with_expansion(self, value: Expansion) -> Self {
        Self {
            expansion: Some(value),
            ..self
        }
    }

    pub fn build(self) -> RecordFormatter {
        let cfg = self.cfg.unwrap_or_default();
        let punctuation = self
            .punctuation
            .unwrap_or_else(|| cfg.punctuation.resolve(self.ascii).into());
        let ts_formatter = self.ts_formatter.unwrap_or_default();
        let ts_width = ts_formatter.max_width();
        let ts_stub = Self::make_ts_stub(&ts_formatter, ts_width.chars);

        RecordFormatter {
            theme: self.theme.unwrap_or_default(),
            unescape_fields: !self.raw_fields,
            ts_formatter,
            ts_width,
            ts_stub,
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
            expansion: self.expansion.unwrap_or_default(),
        }
    }

    fn make_ts_stub(formatter: &DateTimeFormatter, width: usize) -> String {
        let sample = |y, m, d, th, tm, ts, tn| {
            let mut buf = Vec::new();
            let dt = NaiveDateTime::new(
                NaiveDate::from_ymd_opt(y, m, d)?,
                NaiveTime::from_hms_nano_opt(th, tm, ts, tn)?,
            );
            let offset = formatter.tz().offset_from_local_datetime(&dt).earliest()?.fix();
            let dt = DateTime::from_naive_utc_and_offset(dt - offset, offset);
            formatter.format(&mut buf, dt);
            String::from_utf8(buf).ok()
        };

        let s1 = sample(1999, 12, 30, 00, 00, 00, 987_654_321);
        let s2 = sample(1999, 12, 30, 12, 00, 00, 987_654_321);
        let s3 = sample(2011, 7, 2, 9, 48, 48, 712_345_678);

        let (Some(s1), Some(s2), Some(s3)) = (s1, s2, s3) else {
            return "#".repeat(width);
        };

        let mut result = String::new();

        for (c1, c2, c3) in izip!(s1.chars(), s2.chars(), s3.chars()) {
            if c1 == c2 && c2 == c3 {
                result.push(c1);
            } else {
                result.push(TIME_PLACEHOLDER as char);
            }
        }

        result
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
    ts_width: TextWidth,
    ts_stub: String,
    hide_empty_fields: bool,
    flatten: bool,
    always_show_time: bool,
    always_show_level: bool,
    fields: Arc<IncludeExcludeKeyFilter>,
    predefined_fields: Arc<ExactIncludeExcludeKeyFilter>,
    message_format: DynMessageFormat,
    punctuation: Arc<ResolvedPunctuation>,
    expansion: Expansion,
}

impl RecordFormatter {
    pub fn format_record(&self, buf: &mut Buf, prefix: Range<usize>, rec: &model::Record) {
        let mut fs = FormattingStateWithRec {
            rec,
            fs: FormattingState {
                flatten: self.flatten && self.unescape_fields,
                expansion: self.expansion.profile(),
                prefix,
                ..Default::default()
            },
        };

        self.theme.apply(buf, &rec.level, |s| {
            //
            // time
            //
            if fs.transact(s, |fs, s| self.format_timestamp(rec, fs, s)).is_err() {
                if let Some(ts) = &rec.ts {
                    fs.extra_fields
                        .push(("ts", RawValue::String(EncodedString::raw(ts.raw()))))
                        .ok();
                }
                if self.always_show_time {
                    self.format_timestamp_stub(&mut fs, s);
                }
            }

            //
            // level
            //
            let level = match rec.level {
                Some(Level::Error) => Some(LEVEL_ERROR.as_bytes()),
                Some(Level::Warning) => Some(LEVEL_WARNING.as_bytes()),
                Some(Level::Info) => Some(LEVEL_INFO.as_bytes()),
                Some(Level::Debug) => Some(LEVEL_DEBUG.as_bytes()),
                Some(Level::Trace) => Some(LEVEL_TRACE.as_bytes()),
                None => None,
            };
            let level = level.or(self.always_show_level.then_some(LEVEL_UNKNOWN.as_bytes()));
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
                    fs.first_line_used = true;
                });
            }

            //
            // message text
            //
            if let Some(value) = &rec.message {
                match fs.transact(s, |fs, s| self.format_message(s, fs, *value)) {
                    Ok(()) => {
                        fs.first_line_used = true;
                    }
                    Err(MessageFormatError::ExpansionNeeded) => {
                        self.add_field_to_expand(
                            s,
                            &mut fs,
                            "msg",
                            *value,
                            Some(&self.fields),
                            Some(&self.predefined_fields),
                        );
                    }
                    Err(MessageFormatError::FormattingAsFieldNeeded) => {
                        fs.extra_fields.push(("msg", *value)).ok();
                    }
                    Err(MessageFormatError::EmptyMessage) => {}
                }
            } else {
                s.reset();
            }

            //
            // fields
            //
            let mut some_fields_hidden = false;
            let x_fields = take(&mut fs.extra_fields);
            for (k, v) in x_fields.iter().chain(rec.fields()) {
                if !self.hide_empty_fields || !v.is_empty() {
                    let result = fs.transact(s, |fs, s| {
                        match self.format_field(s, k, *v, fs, Some(&self.fields), Some(&self.predefined_fields)) {
                            FieldFormatResult::Ok => {
                                if !fs.expanded {
                                    fs.first_line_used = true;
                                }
                                Ok(())
                            }
                            FieldFormatResult::Hidden => {
                                some_fields_hidden = true;
                                Ok(())
                            }
                            FieldFormatResult::HiddenByPredefined => Ok(()),
                            FieldFormatResult::ExpansionNeeded => Err(()),
                        }
                    });
                    if let Err(()) = result {
                        self.add_field_to_expand(s, &mut fs, k, *v, Some(&self.fields), Some(&self.predefined_fields));
                    }
                }
            }

            //
            // expanded fields
            //
            self.expand_enqueued(s, &mut fs);

            if (some_fields_hidden || (fs.some_nested_fields_hidden && fs.flatten)) || fs.some_fields_hidden {
                if fs.expanded {
                    self.expand(s, &mut fs);
                }
                fs.add_element(|| s.batch(|buf| buf.push(b' ')));
                s.element(Element::Ellipsis, |s| {
                    s.batch(|buf| buf.extend_from_slice(self.punctuation.hidden_fields_indicator.as_bytes()))
                });
            }

            //
            // caller
            //
            if !fs.caller_formatted && !rec.caller.is_empty() {
                self.format_caller(s, &rec.caller);
            }
        });
    }

    #[inline(always)]
    fn format_timestamp<S: StylingPush<Buf>>(
        &self,
        rec: &model::Record,
        fs: &mut FormattingStateWithRec,
        s: &mut S,
    ) -> Result<(), ()> {
        let Some(ts) = &rec.ts else {
            return Err(());
        };

        fs.ts_width = self.ts_width.chars;
        fs.add_element(|| {});
        s.element(Element::Time, |s| {
            s.batch(|buf| {
                aligned_left(buf, self.ts_width.bytes, b' ', |mut buf| {
                    if ts
                        .as_rfc3339()
                        .and_then(|ts| self.ts_formatter.reformat_rfc3339(&mut buf, ts))
                        .is_some()
                    {
                        Ok(())
                    } else if let Some(ts) = ts.parse() {
                        self.ts_formatter.format(&mut buf, ts);
                        Ok(())
                    } else {
                        Err(())
                    }
                })
            })
        })
    }

    #[inline(always)]
    fn format_timestamp_stub<S: StylingPush<Buf>>(&self, fs: &mut FormattingStateWithRec, s: &mut S) {
        fs.ts_width = self.ts_width.chars;
        fs.add_element(|| {});
        s.element(Element::Time, |s| {
            s.batch(|buf| {
                buf.extend_from_slice(self.ts_stub.as_bytes());
            })
        });
    }

    #[inline(always)]
    fn format_caller<S: StylingPush<Buf>>(&self, s: &mut S, caller: &Caller) {
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
    }

    #[inline(always)]
    fn format_field<'a, S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        key: &str,
        value: RawValue<'a>,
        fs: &mut FormattingStateWithRec,
        filter: Option<&IncludeExcludeKeyFilter>,
        predefined_filter: Option<&ExactIncludeExcludeKeyFilter>,
    ) -> FieldFormatResult {
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

    #[inline(always)]
    fn format_message<'a, S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        fs: &mut FormattingStateWithRec,
        value: RawValue<'a>,
    ) -> Result<(), MessageFormatError> {
        match value {
            RawValue::String(value) => {
                if !value.is_empty() {
                    fs.add_element(|| {
                        s.reset();
                        s.space();
                    });
                    s.element(Element::Message, |s| {
                        s.batch(|buf| {
                            let xsa = match fs.expansion.multiline {
                                MultilineExpansion::Disabled => ExtendedSpaceAction::Escape,
                                MultilineExpansion::Standard => ExtendedSpaceAction::Abort,
                                MultilineExpansion::Inline => ExtendedSpaceAction::Inline,
                            };
                            let result = self.message_format.format(value, buf, xsa.into()).unwrap();
                            match result {
                                string::FormatResult::Ok(_) => Ok(()),
                                string::FormatResult::Aborted => Err(MessageFormatError::ExpansionNeeded),
                            }
                        })
                    })
                } else {
                    Err(MessageFormatError::EmptyMessage)
                }
            }
            _ => Err(MessageFormatError::FormattingAsFieldNeeded),
        }
    }

    #[inline(always)]
    fn format_level<S: StylingPush<Buf>>(&self, s: &mut S, fs: &mut FormattingStateWithRec, level: &[u8]) {
        fs.add_element(|| s.space());
        s.element(Element::Level, |s| {
            s.batch(|buf| {
                buf.extend_from_slice(self.punctuation.level_left_separator.as_bytes());
            });
            s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(level)));
            s.batch(|buf| buf.extend_from_slice(self.punctuation.level_right_separator.as_bytes()));
        });
    }

    #[inline(always)]
    fn expand<S: StylingPush<Buf>>(&self, s: &mut S, fs: &mut FormattingStateWithRec) {
        self.expand_impl(s, fs, true);
    }

    #[inline(always)]
    fn expand_enqueued<S: StylingPush<Buf>>(&self, s: &mut S, fs: &mut FormattingStateWithRec) {
        if !fs.fields_to_expand.is_empty() {
            self.expand_impl(s, fs, false);
        }
    }

    #[inline(never)]
    fn expand_impl<S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        fs: &mut FormattingStateWithRec,
        expand_after_enqueued: bool,
    ) {
        if fs.last_expansion_point == Some(s.batch(|buf| buf.len())) {
            return;
        }

        let mut begin = fs.prefix.start;

        if !fs.first_line_used {
            fs.add_element(|| s.space());
            s.element(Element::Message, |s| {
                s.batch(|buf| buf.extend(EXPANDED_MESSAGE_HEADER.as_bytes()));
            });
            fs.first_line_used = true;
        }

        if !fs.caller_formatted {
            if !fs.rec.caller.is_empty() {
                self.format_caller(s, &fs.rec.caller);
            };
            fs.caller_formatted = true;
        }

        s.reset();
        s.batch(|buf| {
            buf.push(b'\n');
            begin = buf.len();
            buf.extend_from_within(fs.prefix.clone());
        });

        fs.dirty = false;
        if fs.ts_width != 0 {
            fs.dirty = true;
            s.element(Element::Time, |s| {
                s.batch(|buf| {
                    aligned_left(buf, fs.ts_width, b' ', |_| {});
                })
            });
        }

        if fs.has_level {
            self.format_level(s, fs, LEVEL_EXPANDED.as_bytes());
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
                buf.extend(EXPANDED_KEY_HEADER.as_bytes());
                fs.dirty = false;
                fs.last_expansion_point = Some(buf.len());
            });
        });

        if !fs.fields_to_expand.is_empty() {
            fs.expanded = true;
            let fields_to_expand = take(&mut fs.fields_to_expand);
            for (k, v) in fields_to_expand.iter() {
                _ = self.format_field(s, k, *v, fs, Some(&self.fields), Some(&self.predefined_fields));
            }
            if expand_after_enqueued {
                self.expand(s, fs);
            }
        }
    }

    #[inline]
    fn add_field_to_expand<'a, S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        fs: &mut FormattingStateWithRec<'a>,
        key: &'a str,
        value: RawValue<'a>,
        filter: Option<&IncludeExcludeKeyFilter>,
        predefined_filter: Option<&ExactIncludeExcludeKeyFilter>,
    ) {
        debug_assert!(!fs.expanded);

        if let Err((key, value)) = fs.fields_to_expand.push((key, value)) {
            self.expand(s, fs);
            _ = self.format_field(s, key, value, fs, filter, predefined_filter);
        }
    }
}

impl RecordWithSourceFormatter for RecordFormatter {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix: Range<usize>, rec: model::RecordWithSource) {
        RecordFormatter::format_record(self, buf, prefix, rec.record)
    }
}

// ---

struct FormattingStateWithRec<'a> {
    fs: FormattingState<'a>,
    rec: &'a model::Record<'a>,
}

impl<'a> FormattingStateWithRec<'a> {
    #[inline(always)]
    fn add_element(&mut self, add_space: impl FnOnce()) {
        if !self.dirty {
            self.dirty = true;
        } else {
            add_space();
        }
    }

    #[inline(always)]
    fn transact<R, E, F>(&mut self, s: &mut Styler<Buf>, f: F) -> Result<R, E>
    where
        F: FnOnce(&mut Self, &mut Styler<Buf>) -> Result<R, E>,
    {
        let dirty = self.dirty;
        let depth = self.depth;
        let first_line_used = self.first_line_used;
        let ts_width = self.ts_width;
        let result = s.transact(|s| f(self, s));
        if result.is_err() {
            self.dirty = dirty;
            self.depth = depth;
            self.first_line_used = first_line_used;
            self.ts_width = ts_width;
        }
        result
    }
}

impl<'a> Deref for FormattingStateWithRec<'a> {
    type Target = FormattingState<'a>;

    #[inline(always)]
    fn deref(&self) -> &FormattingState<'a> {
        &self.fs
    }
}

impl<'a> DerefMut for FormattingStateWithRec<'a> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut FormattingState<'a> {
        &mut self.fs
    }
}

// ---

#[derive(Default)]
struct FormattingState<'a> {
    key_prefix: KeyPrefix,
    flatten: bool,
    some_nested_fields_hidden: bool,
    has_fields: bool,
    expansion: &'a ExpansionProfile,
    expanded: bool,
    prefix: Range<usize>,
    expansion_prefix: Option<Range<usize>>,
    dirty: bool,
    ts_width: usize,
    has_level: bool,
    depth: usize,
    first_line_used: bool,
    some_fields_hidden: bool,
    caller_formatted: bool,
    extra_fields: heapless::Vec<(&'a str, RawValue<'a>), 4>,
    fields_to_expand: heapless::Vec<(&'a str, RawValue<'a>), MAX_FIELDS_TO_EXPAND_ON_HOLD>,
    last_expansion_point: Option<usize>,
}

const MAX_FIELDS_TO_EXPAND_ON_HOLD: usize = 32;

// ---

#[derive(Default)]
struct KeyPrefix {
    value: OptimizedBuf<u8, 256>,
}

impl KeyPrefix {
    #[inline(always)]
    fn len(&self) -> usize {
        self.value.len()
    }

    #[inline(always)]
    fn format<B: Push<u8>>(&self, buf: &mut B) {
        buf.extend_from_slice(self.value.as_slices().0);
        buf.extend_from_slice(self.value.as_slices().1);
    }

    #[inline(always)]
    fn push(&mut self, key: &str) -> usize {
        let len = self.len();
        if len != 0 {
            self.value.push(b'.');
        }
        key.key_prettify(&mut self.value);
        self.len() - len
    }

    #[inline(always)]
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
    #[inline(always)]
    fn new(rf: &'a RecordFormatter) -> Self {
        Self { rf }
    }

    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn format<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        key: &str,
        value: RawValue<'a>,
        fs: &mut FormattingStateWithRec,
        filter: Option<&IncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
        predefined_filter: Option<&ExactIncludeExcludeKeyFilter>,
        predefined_setting: IncludeExcludeSetting,
    ) -> FieldFormatResult {
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
            return FieldFormatResult::HiddenByPredefined;
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
            return FieldFormatResult::Hidden;
        }

        // If expand_all is enabled and we're not already expanded, trigger expansion for all fields
        if !fs.expanded && fs.expansion.expand_all {
            return FieldFormatResult::ExpansionNeeded;
        }

        // For objects with hide_empty_fields or predefined_filter, track buffer position to rollback if empty
        let has_predefined_filter = predefined_filter.is_some();
        let rollback_pos =
            if (self.rf.hide_empty_fields || has_predefined_filter) && matches!(value, RawValue::Object(_)) {
                let mut pos = 0;
                s.batch(|buf| pos = buf.len());
                Some(pos)
            } else {
                None
            };

        let ffv = self.begin(s, key, value, fs);

        let result = if self.rf.unescape_fields {
            self.format_value(s, value, fs, filter, predefined_filter, setting, predefined_setting)
        } else {
            s.element(Element::String, |s| {
                s.batch(|buf| buf.extend(value.raw_str().as_bytes()))
            });
            ValueFormatResult::Ok
        };

        self.end(fs, ffv);

        // If object had no visible content, rollback buffer and state
        if let Some(pos) = rollback_pos {
            if result == ValueFormatResult::Empty {
                s.batch(|buf| buf.truncate(pos));
                return FieldFormatResult::HiddenByPredefined;
            }
        }

        match result {
            ValueFormatResult::Ok | ValueFormatResult::Empty => FieldFormatResult::Ok,
            ValueFormatResult::ExpansionNeeded => FieldFormatResult::ExpansionNeeded,
        }
    }

    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn format_value<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        value: RawValue<'a>,
        fs: &mut FormattingStateWithRec,
        filter: Option<&IncludeExcludeKeyFilter>,
        predefined_filter: Option<&ExactIncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
        predefined_setting: IncludeExcludeSetting,
    ) -> ValueFormatResult {
        let value = match value {
            RawValue::String(EncodedString::Raw(value)) => RawValue::auto(value.as_str()),
            _ => value,
        };

        match value {
            RawValue::String(value) => {
                let result = s.element(Element::String, |s| {
                    s.batch(|buf| {
                        let expand = |buf: &mut Vec<u8>| self.add_prefix(buf, fs);
                        let xsa = match (fs.expanded, fs.expansion.multiline) {
                            (true, _) => ExtendedSpaceAction::Expand(&expand),
                            (false, MultilineExpansion::Inline) => ExtendedSpaceAction::Inline,
                            (false, MultilineExpansion::Disabled) => ExtendedSpaceAction::Escape,
                            (false, MultilineExpansion::Standard) => ExtendedSpaceAction::Abort,
                        };
                        ValueFormatAuto.format(value, buf, xsa.into()).unwrap()
                    })
                });
                match result {
                    string::FormatResult::Ok(_) => {}
                    string::FormatResult::Aborted => {
                        return ValueFormatResult::ExpansionNeeded;
                    }
                }
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
                let item = value.parse().unwrap();
                if !fs.flatten && (!fs.expanded || value.is_empty()) {
                    s.element(Element::Object, |s| {
                        s.batch(|buf| buf.push(b'{'));
                    });
                }
                let mut some_fields_hidden_by_user = false;
                let mut any_fields_formatted = false;
                for (k, v) in item.fields.iter() {
                    if !self.rf.hide_empty_fields || !v.is_empty() {
                        match self.format(s, k, *v, fs, filter, setting, predefined_filter, predefined_setting) {
                            FieldFormatResult::Ok => {
                                any_fields_formatted = true;
                            }
                            FieldFormatResult::Hidden => {
                                some_fields_hidden_by_user = true;
                            }
                            FieldFormatResult::HiddenByPredefined => {}
                            FieldFormatResult::ExpansionNeeded => {
                                return ValueFormatResult::ExpansionNeeded;
                            }
                        }
                    } else {
                        some_fields_hidden_by_user = true;
                    }
                }
                if some_fields_hidden_by_user {
                    if !fs.flatten {
                        if fs.expanded {
                            self.rf.expand(s, fs);
                        }
                        fs.add_element(|| s.batch(|buf| buf.push(b' ')));
                        s.element(Element::Ellipsis, |s| {
                            s.batch(|buf| buf.extend(self.rf.punctuation.hidden_fields_indicator.as_bytes()))
                        });
                    } else {
                        fs.some_fields_hidden = true;
                    }
                }
                if !fs.flatten && (!fs.expanded || value.is_empty()) {
                    s.element(Element::Object, |s| {
                        s.batch(|buf| {
                            if !item.fields.is_empty() {
                                buf.push(b' ');
                            }
                            buf.push(b'}');
                        });
                    });
                }
                fs.some_nested_fields_hidden |= some_fields_hidden_by_user;
                if !any_fields_formatted {
                    // Return Empty if no fields were actually formatted (for rollback support)
                    return ValueFormatResult::Empty;
                }
            }
            RawValue::Array(value) => {
                let xb = replace(&mut fs.expanded, false);
                let inline = fs.expansion.multiline == MultilineExpansion::Inline;
                let saved_expansion = replace(
                    &mut fs.expansion,
                    if inline {
                        &ExpansionProfile::INLINE
                    } else {
                        &ExpansionProfile::NEVER
                    },
                );
                let item = value.parse::<32>().unwrap();
                s.element(Element::Array, |s| {
                    s.batch(|buf| buf.push(b'['));
                    let mut first = true;
                    for v in item.iter() {
                        if !first {
                            s.batch(|buf| buf.extend(self.rf.punctuation.array_separator.as_bytes()));
                        } else {
                            first = false;
                        }
                        _ = self.format_value(
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
                fs.expansion = saved_expansion;
                fs.expanded = xb;
            }
        };

        ValueFormatResult::Ok
    }

    #[inline]
    fn add_prefix(&self, buf: &mut Vec<u8>, fs: &FormattingStateWithRec) -> usize {
        buf.extend(self.rf.theme.expanded_value_suffix.value.as_bytes());
        buf.push(b'\n');
        let prefix = fs.expansion_prefix.clone().unwrap_or(fs.prefix.clone());
        let l0 = buf.len();
        buf.extend_from_within(prefix);
        buf.extend(self.rf.theme.expanded_value_prefix.value.as_bytes());
        buf.len() - l0
    }

    #[inline(always)]
    fn begin<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        key: &str,
        value: RawValue<'a>,
        fs: &mut FormattingStateWithRec,
    ) -> FormattedFieldVariant {
        if fs.flatten && matches!(value, RawValue::Object(_)) {
            return FormattedFieldVariant::Flattened(fs.key_prefix.push(key));
        }

        if !fs.has_fields {
            fs.has_fields = true;
            if self.rf.message_format.delimited && !fs.expanded {
                fs.add_element(|| s.space());
                s.element(Element::MessageDelimiter, |s| {
                    s.batch(|buf| buf.extend(self.rf.punctuation.message_delimiter.as_bytes()));
                });
            }
        }

        let variant = FormattedFieldVariant::Normal { flatten: fs.flatten };

        if fs.expanded {
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

        let sep = if fs.expanded && matches!(value, RawValue::Object(o) if !o.is_empty()) {
            EXPANDED_OBJECT_HEADER.as_bytes()
        } else {
            self.rf.punctuation.field_key_value_separator.as_bytes()
        };

        s.element(Element::Field, |s| {
            s.batch(|buf| buf.extend(sep));
        });

        variant
    }

    #[inline(always)]
    fn end(&mut self, fs: &mut FormattingStateWithRec, v: FormattedFieldVariant) {
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

#[must_use]
#[derive(PartialEq, Eq)]
enum ValueFormatResult {
    Ok,
    Empty,
    ExpansionNeeded,
}

#[must_use]
enum FieldFormatResult {
    Ok,
    Hidden,
    HiddenByPredefined,
    ExpansionNeeded,
}

#[must_use]
enum MessageFormatError {
    ExpansionNeeded,
    FormattingAsFieldNeeded,
    EmptyMessage,
}

// ---

trait WithAutoTrim {
    fn with_auto_trim<F, R>(&mut self, f: F, flags: impl Into<AutoTrimFlags>) -> R
    where
        F: FnOnce(&mut Self) -> R;
}

impl WithAutoTrim for Vec<u8> {
    #[inline(always)]
    fn with_auto_trim<F, R>(&mut self, f: F, flags: impl Into<AutoTrimFlags>) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let flags = flags.into();
        let begin = self.len();
        let result = f(self);
        if let Some(end) = self[begin..].iter().rposition(|&b| !b.is_ascii_whitespace()) {
            self.truncate(begin + end + 1);
        } else if !flags.contains(AutoTrimFlag::PreserveWhiteSpaceOnly) {
            self.truncate(begin);
        }
        result
    }
}

#[derive(EnumSetType, Debug)]
enum AutoTrimFlag {
    PreserveWhiteSpaceOnly,
}

type AutoTrimFlags = EnumSet<AutoTrimFlag>;

// ---

trait KeyPrettify {
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

enum FormattedFieldVariant {
    Normal { flatten: bool },
    Flattened(usize),
}

// ---

pub mod string {
    // std imports
    use std::{cmp::min, ops::Deref, sync::Arc};

    // third-party imports
    use enumset::{EnumSet, EnumSetType, enum_set as mask};
    use memchr::memmem;
    use thiserror::Error;

    // workspace imports
    use encstr::{AnyEncodedString, EncodedString, JsonAppender};
    use enumset_ext::EnumSetExt;
    use mline::prefix_lines_within;

    // local imports
    use crate::{
        formatting::{AutoTrimFlag, AutoTrimFlags, WithAutoTrim},
        model::{MAX_NUMBER_LEN, looks_like_number},
        settings::MessageFormat,
    };

    // ---

    /// Error is an error which may occur in the application.
    #[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Error {
        #[error(transparent)]
        ParseError(#[from] encstr::Error),
    }

    // ---

    pub type Result<T, E = Error> = std::result::Result<T, E>;

    // ---

    pub trait Format {
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            options: FormatOptions<'a>,
        ) -> Result<FormatResult>;

        #[inline(always)]
        fn rtrim(self, n: usize) -> FormatRightTrimmed<Self>
        where
            Self: Sized,
        {
            FormatRightTrimmed::new(n, self)
        }
    }

    pub type DynFormat = Arc<dyn Format + Send + Sync>;

    #[derive(Clone, Copy)]
    pub struct FormatOptions<'a> {
        xsa: ExtendedSpaceAction<'a>,
    }

    impl<'a> From<ExtendedSpaceAction<'a>> for FormatOptions<'a> {
        #[inline(always)]
        fn from(xsa: ExtendedSpaceAction<'a>) -> Self {
            Self { xsa }
        }
    }

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

    impl Deref for DynMessageFormat {
        type Target = DynFormat;

        #[inline(always)]
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

    pub trait Analyze {
        fn analyze(&self) -> Analysis;
    }

    impl Analyze for [u8] {
        #[inline(always)]
        fn analyze(&self) -> Analysis {
            let mut chars = Mask::empty();
            self.iter().map(|&c| CHAR_GROUPS[c as usize]).for_each(|group| {
                chars |= group;
            });
            Analysis { chars }
        }
    }

    // ---

    #[derive(Clone, Copy)]
    pub enum ExtendedSpaceAction<'a> {
        Inline,
        Expand(&'a dyn Fn(&mut Vec<u8>) -> usize),
        Escape,
        Abort,
    }

    #[must_use]
    pub enum FormatResult {
        Ok(Option<Analysis>),
        Aborted,
    }

    impl FormatResult {
        #[cfg(test)]
        #[inline(always)]
        pub fn is_ok(&self) -> bool {
            matches!(self, Self::Ok(_))
        }
    }

    // ---

    pub struct Analysis {
        pub chars: Mask,
    }

    impl Analysis {
        #[inline(always)]
        pub fn empty() -> Self {
            Self { chars: Mask::empty() }
        }
    }

    // ---

    // ---

    pub trait DisplayResolve {
        type Output: std::fmt::Display;

        fn resolve(self) -> Self::Output;
    }

    impl<'a> DisplayResolve for &'a str {
        type Output = Self;

        #[inline(always)]
        fn resolve(self) -> &'a str {
            self
        }
    }

    impl<'a, F> DisplayResolve for F
    where
        F: FnOnce() -> &'a str,
    {
        type Output = &'a str;

        #[inline(always)]
        fn resolve(self) -> &'a str {
            self()
        }
    }

    // ---

    #[derive(Default)]
    pub struct ValueFormatAuto;

    impl Format for ValueFormatAuto {
        #[inline(always)]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            options: FormatOptions<'a>,
        ) -> Result<FormatResult> {
            if input.is_empty() {
                buf.extend(r#""""#.as_bytes());
                return Ok(FormatResult::Ok(None));
            }

            let begin = buf.len();
            _ = buf.with_auto_trim(
                |buf| ValueFormatRaw.format(input, buf, options),
                AutoTrimFlag::PreserveWhiteSpaceOnly,
            )?;

            let analysis = buf[begin..].analyze();
            let mask = analysis.chars;

            const NON_PLAIN: Mask = mask!(
                Flag::Control
                    | Flag::DoubleQuote
                    | Flag::SingleQuote
                    | Flag::Backslash
                    | Flag::Backtick
                    | Flag::Space
                    | Flag::Tab
                    | Flag::Newline
                    | Flag::EqualSign
            );

            let confusing = || matches!(&buf[begin..], [b'{', ..] | [b'[', ..] | b"true" | b"false" | b"null");

            let like_number = || {
                (mask == mask!(Flag::Digit) && buf[begin..].len() <= MAX_NUMBER_LEN) || looks_like_number(&buf[begin..])
            };

            if !mask.intersects(NON_PLAIN) && !confusing() && !like_number() {
                return Ok(FormatResult::Ok(None));
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(None));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(None));
            }

            const WS: Mask = mask!(Flag::Newline | Flag::Tab | Flag::Space);

            let has_control = mask.contains(Flag::Control);
            let has_backtick = mask.contains(Flag::Backtick);
            let has_extended_space = mask.intersects(Flag::Newline | Flag::Tab);
            let has_non_whitespace = mask.intersects(!WS);

            if !has_control && has_non_whitespace {
                if !has_backtick && (!has_extended_space || matches!(options.xsa, ExtendedSpaceAction::Inline)) {
                    buf.push(b'`');
                    buf.push(b'`');
                    buf[begin..].rotate_right(1);
                    return Ok(FormatResult::Ok(None));
                }

                match options.xsa {
                    ExtendedSpaceAction::Expand(prefix) => {
                        let l0 = buf.len();
                        let pl = prefix(buf);
                        let n = buf.len() - l0;
                        buf[begin..].rotate_right(n);
                        prefix_lines_within(buf, begin + n.., 1.., (begin + n - pl)..(begin + n));
                        return Ok(FormatResult::Ok(None));
                    }
                    ExtendedSpaceAction::Abort => {
                        buf.truncate(begin);
                        return Ok(FormatResult::Aborted);
                    }
                    _ => {}
                }
            }

            buf.truncate(begin);
            ValueFormatDoubleQuoted.format(input, buf, options)
        }
    }

    // ---

    pub struct ValueFormatRaw;

    impl Format for ValueFormatRaw {
        #[inline(always)]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            _: FormatOptions<'a>,
        ) -> Result<FormatResult> {
            input.decode(buf)?;
            Ok(FormatResult::Ok(None))
        }
    }

    // ---

    pub struct ValueFormatDoubleQuoted;

    impl Format for ValueFormatDoubleQuoted {
        #[inline(always)]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            _: FormatOptions<'a>,
        ) -> Result<FormatResult> {
            input.format_json(buf)?;
            Ok(FormatResult::Ok(None))
        }
    }

    // ---

    pub struct MessageFormatAutoQuoted;

    impl Format for MessageFormatAutoQuoted {
        #[inline(always)]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            options: FormatOptions<'a>,
        ) -> Result<FormatResult> {
            if input.is_empty() {
                buf.extend(r#""""#.as_bytes());
                return Ok(FormatResult::Ok(Some(Analysis::empty())));
            }

            let begin = buf.len();
            _ = buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf, options), AutoTrimFlags::new())?;

            let analysis = buf[begin..].analyze();
            let mask = analysis.chars;

            const NOT_PLAIN: Mask = mask!(Flag::EqualSign | Flag::Control | Flag::Newline | Flag::Backslash);

            if !mask.intersects(NOT_PLAIN)
                && !matches!(buf.get(begin), Some(b'"' | b'\'' | b'`'))
                && &buf[begin..] != b"~"
            {
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            const XS: Mask = mask!(Flag::Newline);
            let has_control = mask.contains(Flag::Control);
            let has_backtick = mask.contains(Flag::Backtick);
            let has_newline = mask.intersects(XS);

            if !has_control {
                if !has_backtick && (!has_newline || matches!(options.xsa, ExtendedSpaceAction::Inline)) {
                    buf.push(b'`');
                    buf.push(b'`');
                    buf[begin..].rotate_right(1);
                    return Ok(FormatResult::Ok(Some(analysis)));
                }

                if matches!(options.xsa, ExtendedSpaceAction::Abort) {
                    buf.truncate(begin);
                    return Ok(FormatResult::Aborted);
                }
            }

            buf.truncate(begin);
            MessageFormatDoubleQuoted.format(input, buf, options)
        }
    }

    // ---

    pub struct MessageFormatAlwaysQuoted;

    impl Format for MessageFormatAlwaysQuoted {
        #[inline(always)]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            options: FormatOptions<'a>,
        ) -> Result<FormatResult> {
            if input.is_empty() {
                return Ok(FormatResult::Ok(Some(Analysis::empty())));
            }

            let begin = buf.len();
            buf.push(b'"');
            _ = buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf, options), AutoTrimFlags::new())?;

            let analysis = buf[begin + 1..].analyze();
            let mask = analysis.chars;

            const XS: Mask = mask!(Flag::Newline);
            let has_newline = mask.intersects(XS);

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
                buf.push(b'"');
                return Ok(FormatResult::Ok(None));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
                buf[begin] = b'\'';
                buf.push(b'\'');
                return Ok(FormatResult::Ok(None));
            }

            if !mask.intersects(Flag::Backtick | Flag::Control) {
                // Backticks can contain newlines, but only in inline mode
                if !has_newline || matches!(options.xsa, ExtendedSpaceAction::Inline) {
                    buf[begin] = b'`';
                    buf.push(b'`');
                    return Ok(FormatResult::Ok(None));
                }

                // For newlines in non-inline modes, fall through to double-quoted or abort
                if matches!(options.xsa, ExtendedSpaceAction::Abort) {
                    buf.truncate(begin);
                    return Ok(FormatResult::Aborted);
                }
            }

            buf.truncate(begin);
            MessageFormatDoubleQuoted.format(input, buf, options)
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
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            options: FormatOptions<'a>,
        ) -> Result<FormatResult> {
            if input.is_empty() {
                return Ok(FormatResult::Ok(None));
            }

            let begin = buf.len();
            _ = buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf, options), AutoTrimFlags::new())?;

            let analysis = buf[begin..].analyze();
            let mask = analysis.chars;

            const XS: Mask = mask!(Flag::Newline);
            if mask.intersects(XS) && matches!(options.xsa, ExtendedSpaceAction::Abort) {
                buf.truncate(begin);
                return Ok(FormatResult::Aborted);
            }

            let can_inline = !mask.intersects(XS) || matches!(options.xsa, ExtendedSpaceAction::Inline);

            if !mask.contains(Flag::Control)
                && !matches!(buf.get(begin), Some(b'"' | b'\'' | b'`'))
                && &buf[begin..] != b"~"
                && memmem::find(&buf[begin..], self.0.as_bytes()).is_none()
                && can_inline
            {
                buf.extend(self.0.as_bytes());
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                buf.extend(self.0.as_bytes());
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                buf.extend(self.0.as_bytes());
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::Backtick | Flag::Control) && can_inline {
                buf.push(b'`');
                buf.push(b'`');
                buf[begin..].rotate_right(1);
                buf.extend(self.0.as_bytes());
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            buf.truncate(begin);
            let result = MessageFormatDoubleQuoted.format(input, buf, options)?;
            buf.extend(self.0.as_bytes());
            Ok(result)
        }
    }

    // ---

    pub struct MessageFormatRaw;

    impl Format for MessageFormatRaw {
        #[inline(always)]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            _: FormatOptions<'a>,
        ) -> Result<FormatResult> {
            input.decode(buf)?;
            Ok(FormatResult::Ok(None))
        }
    }

    // ---

    pub struct MessageFormatDoubleQuoted;

    impl Format for MessageFormatDoubleQuoted {
        #[inline(always)]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            _: FormatOptions<'a>,
        ) -> Result<FormatResult> {
            input.format_json(buf)?;
            Ok(FormatResult::Ok(None))
        }
    }

    // ---

    pub struct FormatRightTrimmed<F> {
        n: usize,
        inner: F,
    }

    impl<F> FormatRightTrimmed<F> {
        #[inline(always)]
        fn new(n: usize, inner: F) -> Self {
            Self { n, inner }
        }
    }

    impl<F: Format> Format for FormatRightTrimmed<F> {
        #[inline(always)]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            options: FormatOptions<'a>,
        ) -> Result<FormatResult> {
            let begin = buf.len();
            let result = self.inner.format(input, buf, options)?;
            buf.truncate(buf.len() - min(buf.len() - begin, self.n));
            Ok(result)
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
        #[inline(always)]
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
        const NL: Mask = mask!(Flag::Newline); // 0x0A, 0x0D
        const EQ: Mask = mask!(Flag::EqualSign); // 0x3D
        const HY: Mask = mask!(Flag::Minus); // Hyphen, 0x2D
        const DO: Mask = mask!(Flag::Dot); // Dot, 0x2E
        const DD: Mask = mask!(Flag::Digit); // Decimal digit, 0x30..0x39
        const CL: Mask = mask!(Flag::Colon); // Colon, 0x3A
        const TL: Mask = mask!(Flag::Tilde); // Tilde, 0x7E
        const PA: Mask = mask!(Flag::Parantheses); // 0x28, 0x29
        const BK: Mask = mask!(Flag::Brackets); // 0x5B, 0x5D
        const BR: Mask = mask!(Flag::Braces); // 0x7B, 0x7D
        const AB: Mask = mask!(Flag::AngleBrackets); // 0x3C, 0x3E
        const __: Mask = mask!(Flag::Other);
        [
            //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
            CT, CT, CT, CT, CT, CT, CT, CT, CT, TB, NL, CT, CT, NL, CT, CT, // 0
            CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 1
            SP, __, DQ, __, __, __, __, SQ, __, __, __, __, __, HY, DO, __, // 2
            DD, DD, DD, DD, DD, DD, DD, DD, DD, DD, CL, __, AB, EQ, AB, __, // 3
            __, __, __, __, __, __, __, __, PA, PA, __, __, __, __, __, __, // 4
            __, __, __, __, __, __, __, __, __, __, __, BK, BS, BK, __, __, // 5
            BT, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
            __, __, __, __, __, __, __, __, __, __, __, BR, __, BR, TL, __, // 7
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
    pub enum Flag {
        Control,
        DoubleQuote,
        SingleQuote,
        Backslash,
        Backtick,
        Space,
        Tab,
        Newline,
        EqualSign,
        Digit,
        Minus,
        Dot,
        Colon,
        Tilde,
        Parantheses,
        Brackets,
        Braces,
        AngleBrackets,
        Other,
    }

    pub type Mask = EnumSet<Flag>;
}

#[cfg(test)]
mod tests;
