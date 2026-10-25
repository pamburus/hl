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
    diagnostics::UnparsedKind,
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
use string::{DynMessageFormat, ExtendedSpaceAction, Format, KeyQuoting, ValueFormatAuto};

// ---

pub mod string;
pub mod structured;

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

    /// Writes input data that is not a record, or leaves it out and passes it to `sink`,
    /// and returns whether it is written.
    ///
    /// The `prefix` goes before each line of the data and the `delimiter` between its lines.
    /// By default, the data is written as is unless it is hidden.
    #[inline]
    fn format_unparsed(
        &self,
        buf: &mut Buf,
        prefix: &[u8],
        delimiter: &[u8],
        unparsed: UnparsedData,
        _: &mut dyn DropSink,
    ) -> bool {
        if !unparsed.hidden {
            write_unparsed(buf, prefix, delimiter, unparsed.data, unparsed.kind);
        }
        !unparsed.hidden
    }

    /// Returns whether the output carries decorations, such as input badges and sync indicators.
    #[inline]
    fn decorations(&self) -> bool {
        true
    }
}

/// Input data that is not a record.
pub struct UnparsedData<'a> {
    pub data: &'a [u8],
    /// Where the data was read from.
    pub origin: model::Origin<'a>,
    pub kind: UnparsedKind,
    /// Whether the data is hidden, for example by filtering, so that output showing such data leaves it out.
    pub hidden: bool,
}

/// Takes input data left out of the output, as it is not a record.
pub trait DropSink {
    fn take(&mut self, unparsed: UnparsedData);
}

/// Takes dropped data and does nothing with it.
impl DropSink for () {
    #[inline]
    fn take(&mut self, _: UnparsedData) {}
}

/// Writes input data that is not a record as is, each of its lines with `prefix`.
///
/// An entry ends with `delimiter`. Data before a record ends with a space that sets it apart
/// from the record, and data after the records continues their line, so neither of them ends
/// with a delimiter. A fragment, a part of an entry, is written after `prefix` as is.
fn write_unparsed(buf: &mut Buf, prefix: &[u8], delimiter: &[u8], data: &[u8], kind: UnparsedKind) {
    let searcher = Newline.into_searcher();
    let lines = searcher.split(data);
    match kind {
        UnparsedKind::Fragment => {
            buf.extend(prefix);
            buf.extend(data);
        }
        // An empty entry is written as an empty line.
        UnparsedKind::Entry if data.is_empty() => {
            buf.extend(prefix);
            buf.extend(delimiter);
        }
        UnparsedKind::Entry => {
            for line in lines {
                buf.extend(prefix);
                buf.extend(line);
                buf.extend(delimiter);
            }
        }
        UnparsedKind::Prefix | UnparsedKind::Remainder => {
            for (i, line) in lines.enumerate() {
                if i != 0 {
                    buf.extend(delimiter);
                }
                // The first line of data after the records continues their line.
                if i != 0 || kind == UnparsedKind::Prefix {
                    buf.extend(prefix);
                }
                buf.extend(line);
            }
            if kind == UnparsedKind::Prefix && data.last() != Some(&b' ') {
                buf.push(b' ');
            }
        }
    }
}

pub struct RawRecordFormatter {
    pub delimiter: String,
}

impl RecordWithSourceFormatter for RawRecordFormatter {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix: Range<usize>, rec: model::RecordWithSource) {
        let mut first = true;
        for line in Newline.into_searcher().split(rec.source) {
            if !first {
                buf.extend_from_slice(self.delimiter.as_bytes());
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

    #[inline(always)]
    fn format_unparsed(
        &self,
        buf: &mut Buf,
        prefix: &[u8],
        delimiter: &[u8],
        unparsed: UnparsedData,
        sink: &mut dyn DropSink,
    ) -> bool {
        (**self).format_unparsed(buf, prefix, delimiter, unparsed, sink)
    }

    #[inline(always)]
    fn decorations(&self) -> bool {
        (**self).decorations()
    }
}

impl<T: RecordWithSourceFormatter + ?Sized> RecordWithSourceFormatter for Arc<T> {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix: Range<usize>, rec: model::RecordWithSource) {
        (**self).format_record(buf, prefix, rec)
    }

    #[inline(always)]
    fn format_unparsed(
        &self,
        buf: &mut Buf,
        prefix: &[u8],
        delimiter: &[u8],
        unparsed: UnparsedData,
        sink: &mut dyn DropSink,
    ) -> bool {
        (**self).format_unparsed(buf, prefix, delimiter, unparsed, sink)
    }

    #[inline(always)]
    fn decorations(&self) -> bool {
        (**self).decorations()
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
            prettify_field_keys: !self.raw_fields && cfg.prettify_field_keys.unwrap_or(true),
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
    prettify_field_keys: bool,
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
    fn push(&mut self, key: &str, prettify: bool, quote: bool) -> usize {
        let len = self.len();
        if len != 0 {
            self.value.push(b'.');
        }
        push_key(&mut self.value, key, prettify, quote);
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
            return FormattedFieldVariant::Flattened(fs.key_prefix.push(
                key,
                self.rf.prettify_field_keys,
                self.rf.unescape_fields,
            ));
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
                push_key(buf, key, self.rf.prettify_field_keys, self.rf.unescape_fields);
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

impl KeyPrettify for [u8] {
    #[inline(always)]
    fn key_prettify<B: Push<u8>>(&self, buf: &mut B) {
        let bytes = self;
        let mut i = 0;
        while let Some(pos) = bytes[i..].iter().position(|&b| b == b'_') {
            buf.extend_from_slice(&bytes[i..i + pos]);
            buf.push(b'-');
            i += pos + 1;
        }
        buf.extend_from_slice(&bytes[i..])
    }
}

/// Appends a field key to `buf`, prettified if `prettify` is set,
/// and quoted if `quote` is set and the key would otherwise be ambiguous.
#[inline(always)]
fn push_key<B: Push<u8>>(buf: &mut B, key: &str, prettify: bool, quote: bool) {
    let quoting = if quote {
        KeyQuoting::of(key.as_bytes())
    } else {
        KeyQuoting::Plain
    };

    match quoting {
        KeyQuoting::Plain => push_key_plain(buf, key.as_bytes(), prettify),
        KeyQuoting::Quoted(q) => {
            buf.push(q.byte());
            push_key_plain(buf, key.as_bytes(), prettify);
            buf.push(q.byte());
        }
        KeyQuoting::JsonEscaped => {
            // Rare path: keys with control characters, so an allocation is acceptable here.
            // JSON escape sequences never contain `_`, so prettifying after escaping gives the same result.
            let mut tmp = Vec::with_capacity(key.len() + 8);
            string::format_json_str(key, &mut tmp);
            push_key_plain(buf, &tmp, prettify);
        }
    }
}

#[inline(always)]
fn push_key_plain<B: Push<u8>>(buf: &mut B, key: &[u8], prettify: bool) {
    if prettify {
        key.key_prettify(buf);
    } else {
        buf.extend_from_slice(key);
    }
}

// ---

enum FormattedFieldVariant {
    Normal { flatten: bool },
    Flattened(usize),
}

#[cfg(test)]
mod tests;
