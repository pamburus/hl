// std imports
use std::{
    ops::{Deref, DerefMut, Range},
    sync::Arc,
};

// workspace imports
use encstr::EncodedString;

// local imports
use crate::{
    IncludeExcludeKeyFilter,
    datefmt::{DateTimeFormatter, TextWidth},
    filtering::IncludeExcludeSetting,
    fmtx::{OptimizedBuf, Push, aligned_left, centered},
    model::{self, Caller, Level, RawValue},
    settings::{self, AsciiMode, ExpansionMode, Formatting, MultilineExpansion, ResolvedPunctuation},
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

const DEFAULT_EXPANSION_LOW_THRESHOLDS: ExpansionThresholds = ExpansionThresholds {
    global: 4096,
    cumulative: 512,
    message: 256,
    field: 128,
};

const DEFAULT_EXPANSION_MEDIUM_THRESHOLDS: ExpansionThresholds = ExpansionThresholds {
    global: 2048,
    cumulative: 256,
    message: 192,
    field: 64,
};

const DEFAULT_EXPANSION_HIGH_THRESHOLDS: ExpansionThresholds = ExpansionThresholds {
    global: 1024,
    cumulative: 192,
    message: 128,
    field: 48,
};

// ---

#[derive(Clone, Debug, Default)]
pub struct Expansion {
    pub mode: ExpansionMode,
    pub profiles: ExpansionProfiles,
}

impl Expansion {
    pub fn with_mode(mut self, mode: ExpansionMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn profile(&self) -> &ExpansionProfile {
        self.profiles.resolve(self.mode)
    }
}

impl From<settings::ExpansionOptions> for Expansion {
    fn from(options: settings::ExpansionOptions) -> Self {
        Self {
            mode: options.mode.unwrap_or_default(),
            profiles: options.profiles.into(),
        }
    }
}

impl From<settings::ExpansionMode> for Expansion {
    fn from(mode: settings::ExpansionMode) -> Self {
        Self {
            mode: mode,
            profiles: Default::default(),
        }
    }
}

// ---

#[derive(Clone, Debug, Default)]
pub struct ExpansionProfiles {
    pub low: ExpansionProfileLow,
    pub medium: ExpansionProfileMedium,
    pub high: ExpansionProfileHigh,
}

impl ExpansionProfiles {
    pub fn resolve(&self, mode: ExpansionMode) -> &ExpansionProfile {
        match mode {
            ExpansionMode::Never => &ExpansionProfile::NEVER,
            ExpansionMode::Always => &ExpansionProfile::ALWAYS,
            ExpansionMode::Inline => &ExpansionProfile::INLINE,
            ExpansionMode::Low => &self.low,
            ExpansionMode::Medium => &self.medium,
            ExpansionMode::High => &self.high,
        }
    }
}

impl From<settings::ExpansionProfiles> for ExpansionProfiles {
    fn from(options: settings::ExpansionProfiles) -> Self {
        Self {
            low: options.low.into(),
            medium: options.medium.into(),
            high: options.high.into(),
        }
    }
}

// ---

#[derive(Clone, Debug)]
pub struct ExpansionProfileLow(ExpansionProfile);

impl From<settings::ExpansionProfile> for ExpansionProfileLow {
    fn from(options: settings::ExpansionProfile) -> Self {
        Self(Self::default().0.updated(options))
    }
}

impl Deref for ExpansionProfileLow {
    type Target = ExpansionProfile;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ExpansionProfileLow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for ExpansionProfileLow {
    fn default() -> Self {
        Self(ExpansionProfile {
            multiline: MultilineExpansion::Standard,
            thresholds: DEFAULT_EXPANSION_LOW_THRESHOLDS,
        })
    }
}

// ---

#[derive(Clone, Debug)]
pub struct ExpansionProfileMedium(ExpansionProfile);

impl From<settings::ExpansionProfile> for ExpansionProfileMedium {
    fn from(options: settings::ExpansionProfile) -> Self {
        Self(Self::default().0.updated(options))
    }
}

impl Deref for ExpansionProfileMedium {
    type Target = ExpansionProfile;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ExpansionProfileMedium {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for ExpansionProfileMedium {
    fn default() -> Self {
        Self(ExpansionProfile {
            multiline: MultilineExpansion::Standard,
            thresholds: DEFAULT_EXPANSION_MEDIUM_THRESHOLDS,
        })
    }
}

// ---

#[derive(Clone, Debug)]
pub struct ExpansionProfileHigh(ExpansionProfile);

impl From<settings::ExpansionProfile> for ExpansionProfileHigh {
    fn from(options: settings::ExpansionProfile) -> Self {
        Self(Self::default().0.updated(options))
    }
}

impl Deref for ExpansionProfileHigh {
    type Target = ExpansionProfile;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ExpansionProfileHigh {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for ExpansionProfileHigh {
    fn default() -> Self {
        Self(ExpansionProfile {
            multiline: MultilineExpansion::Standard,
            thresholds: DEFAULT_EXPANSION_HIGH_THRESHOLDS,
        })
    }
}

// ---

#[derive(Clone, Debug)]
pub struct ExpansionProfile {
    pub multiline: MultilineExpansion,
    pub thresholds: ExpansionThresholds,
}

impl ExpansionProfile {
    pub const NEVER: Self = Self {
        multiline: MultilineExpansion::Disabled,
        thresholds: ExpansionThresholds {
            global: usize::MAX,
            cumulative: usize::MAX,
            message: usize::MAX,
            field: usize::MAX,
        },
    };

    pub const ALWAYS: Self = Self {
        multiline: MultilineExpansion::Standard,
        thresholds: ExpansionThresholds {
            global: 0,
            cumulative: 0,
            message: 256,
            field: 0,
        },
    };

    pub const INLINE: Self = Self {
        multiline: MultilineExpansion::Inline,
        thresholds: ExpansionThresholds {
            global: usize::MAX,
            cumulative: usize::MAX,
            message: usize::MAX,
            field: usize::MAX,
        },
    };

    fn update(&mut self, options: settings::ExpansionProfile) {
        self.multiline = options.multiline.unwrap_or(self.multiline);
        self.thresholds.update(&options.thresholds);
    }

    fn updated(mut self, options: settings::ExpansionProfile) -> Self {
        self.update(options);
        self
    }
}

impl Default for &ExpansionProfile {
    fn default() -> Self {
        &ExpansionProfile::NEVER
    }
}

// ---

#[derive(Clone, Debug)]
pub struct ExpansionThresholds {
    pub global: usize,
    pub cumulative: usize,
    pub message: usize,
    pub field: usize,
}

impl ExpansionThresholds {
    fn update(&mut self, options: &settings::ExpansionThresholds) {
        self.global = options.global.unwrap_or(self.global);
        self.cumulative = options.cumulative.unwrap_or(self.cumulative);
        self.message = options.message.unwrap_or(self.message);
        self.field = options.field.unwrap_or(self.field);
    }
}

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

impl<T: RecordWithSourceFormatter + ?Sized> RecordWithSourceFormatter for &T {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: model::RecordWithSource) {
        (**self).format_record(buf, prefix_range, rec)
    }
}

impl<T: RecordWithSourceFormatter + ?Sized> RecordWithSourceFormatter for Arc<T> {
    #[inline(always)]
    fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: model::RecordWithSource) {
        (**self).format_record(buf, prefix_range, rec)
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
            message_format: self
                .message_format
                .unwrap_or_else(|| DynMessageFormat::new(&cfg, self.ascii)),
            punctuation,
            expansion: self.expansion.unwrap_or_default(),
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
    ts_width: TextWidth,
    hide_empty_fields: bool,
    flatten: bool,
    always_show_time: bool,
    always_show_level: bool,
    fields: Arc<IncludeExcludeKeyFilter>,
    message_format: DynMessageFormat,
    punctuation: Arc<ResolvedPunctuation>,
    expansion: Expansion,
}

impl RecordFormatter {
    pub fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: &model::Record) {
        let mut fs = FormattingStateWithRec {
            rec,
            fs: FormattingState {
                flatten: self.flatten && self.unescape_fields,
                expansion: self.expansion.profile(),
                prefix: prefix_range,
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
                } else if self.always_show_time {
                    self.format_timestamp_stub(&mut fs, s);
                    fs.complexity += 1 + self.ts_width.chars;
                }
            } else {
                fs.complexity += 1 + self.ts_width.chars;
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
                fs.complexity += 3 + level.len();
                self.format_level(s, &mut fs, level);
                // fs.add_element(|| s.space());
                // s.element(Element::Level, |s| {
                //     s.batch(|buf| {
                //         buf.extend_from_slice(self.punctuation.level_left_separator.as_bytes());
                //     });
                //     s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(level)));
                //     s.batch(|buf| buf.extend_from_slice(self.punctuation.level_right_separator.as_bytes()));
                // });
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
                    fs.complexity += 2 + logger.len();
                    fs.first_line_used = true;
                });
            }

            // include caller into cumulative complexity calculation
            if !rec.caller.is_empty() {
                fs.complexity += 3 + rec.caller.name.len() + 1 + rec.caller.file.len() + 1 + rec.caller.line.len();
            }

            //
            // message text
            //
            if let Some(value) = &rec.message {
                match fs.transact(s, |fs, s| self.format_message(s, fs, *value)) {
                    Ok(()) => {
                        fs.complexity += 2;
                        fs.first_line_used = true;
                    }
                    Err(MessageFormatError::ExpansionNeeded) => {
                        self.add_field_to_expand(s, &mut fs, "msg", *value, Some(&self.fields));
                    }
                    Err(MessageFormatError::FormattingAsFieldNeeded) => {
                        fs.extra_fields.push(("msg", *value)).ok();
                    }
                    Err(MessageFormatError::EmptyMessage) => {}
                }
            } else {
                s.reset();
            }

            match (fs.expansion.thresholds.global, fs.expansion.thresholds.cumulative) {
                (0, 0) => {}
                (usize::MAX, usize::MAX) => {}
                (global, cumulative) => {
                    if fs.complexity >= cumulative {
                        fs.expanded = true;
                    } else if self.rough_complexity(fs.complexity, rec, Some(&self.fields)) >= global {
                        fs.expanded = true;
                    }
                }
            }

            //
            // fields
            //
            let mut some_fields_hidden = false;
            let x_fields = std::mem::take(&mut fs.extra_fields);
            for (k, v) in x_fields.iter().chain(rec.fields()) {
                if !self.hide_empty_fields || !v.is_empty() {
                    let result = fs.transact(s, |fs, s| match self.format_field(s, k, *v, fs, Some(&self.fields)) {
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
                        FieldFormatResult::ExpansionNeeded => Err(()),
                    });
                    if let Err(()) = result {
                        self.add_field_to_expand(s, &mut fs, k, *v, Some(&self.fields));
                    }
                }
            }

            //
            // expanded fields
            //
            if fs.fields_to_expand.len() != 0 {
                self.expand(s, &mut fs);
            }

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
            if !fs.caller_formatted {
                if !rec.caller.is_empty() {
                    self.format_caller(s, &rec.caller);
                };
            }
            // if !rec.caller.is_empty() {
            //     let caller = rec.caller;
            //     s.element(Element::Caller, |s| {
            //         s.batch(|buf| {
            //             buf.push(b' ');
            //             buf.extend(self.punctuation.source_location_separator.as_bytes())
            //         });
            //         s.element(Element::CallerInner, |s| {
            //             s.batch(|buf| {
            //                 if !caller.name.is_empty() {
            //                     buf.extend(caller.name.as_bytes());
            //                 }
            //                 if !caller.file.is_empty() || !caller.line.is_empty() {
            //                     if !caller.name.is_empty() {
            //                         buf.extend(self.punctuation.caller_name_file_separator.as_bytes());
            //                     }
            //                     buf.extend(caller.file.as_bytes());
            //                     if !caller.line.is_empty() {
            //                         buf.push(b':');
            //                         buf.extend(caller.line.as_bytes());
            //                     }
            //                 }
            //             });
            //         });
            //     });
            // };
        });
    }

    #[inline]
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

    #[inline]
    fn format_timestamp_stub<S: StylingPush<Buf>>(&self, fs: &mut FormattingStateWithRec, s: &mut S) {
        fs.ts_width = self.ts_width.chars;
        fs.add_element(|| {});
        s.element(Element::Time, |s| {
            s.batch(|buf| {
                centered(buf, self.ts_width.chars, b'-', |mut buf| {
                    buf.extend_from_slice(b"-");
                });
            })
        });
    }

    #[inline]
    fn rough_complexity(&self, initial: usize, rec: &model::Record, filter: Option<&IncludeExcludeKeyFilter>) -> usize {
        let mut result = initial;
        result += rec.message.map(|x| x.raw_str().len()).unwrap_or(0);
        result += rec.predefined.len();
        result += rec.logger.map(|x| x.len()).unwrap_or(0);
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

            result += 2 + key.len();
            result += value.rough_complexity();
        }
        result
    }

    #[inline]
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

    #[inline]
    fn format_field<'a, S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        key: &str,
        value: RawValue<'a>,
        fs: &mut FormattingStateWithRec,
        filter: Option<&IncludeExcludeKeyFilter>,
    ) -> FieldFormatResult {
        let mut fv = FieldFormatter::new(self);
        fv.format(s, key, value, fs, filter, IncludeExcludeSetting::Unspecified)
    }

    #[inline]
    fn format_message<'a, S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        fs: &mut FormattingStateWithRec,
        value: RawValue<'a>,
    ) -> Result<(), MessageFormatError> {
        match value {
            RawValue::String(value) => {
                if !value.is_empty() {
                    if value.source().len() > fs.expansion.thresholds.message {
                        return Err(MessageFormatError::ExpansionNeeded);
                    }
                    fs.add_element(|| {
                        s.reset();
                        s.space();
                    });
                    s.element(Element::Message, |s| {
                        s.batch(|buf| {
                            let xsa = match (fs.expanded, fs.expansion.multiline) {
                                (true, _) => ExtendedSpaceAction::Abort,
                                (false, MultilineExpansion::Disabled) => ExtendedSpaceAction::Escape,
                                (false, MultilineExpansion::Standard) => ExtendedSpaceAction::Abort,
                                (false, MultilineExpansion::Inline) => ExtendedSpaceAction::Inline,
                            };
                            let result = self.message_format.format(value, buf, xsa).unwrap();
                            match result {
                                string::FormatResult::Ok(analysis) => {
                                    if let Some(analysis) = analysis {
                                        fs.complexity += analysis.complexity;
                                    }
                                    Ok(())
                                }
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

    #[inline]
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

    fn expand<S: StylingPush<Buf>>(&self, s: &mut S, fs: &mut FormattingStateWithRec) {
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

        if !fs.expanded {
            fs.expanded = true;
            let fields_to_expand = std::mem::take(&mut fs.fields_to_expand);
            for (k, v) in fields_to_expand.iter() {
                _ = self.format_field(s, k, *v, fs, Some(&self.fields));
            }
        }
    }

    fn add_field_to_expand<'a, S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        fs: &mut FormattingStateWithRec<'a>,
        key: &'a str,
        value: RawValue<'a>,
        filter: Option<&IncludeExcludeKeyFilter>,
    ) {
        let result = if fs.expanded {
            Err((key, value))
        } else {
            fs.fields_to_expand.push((key, value))
        };

        if let Err((key, value)) = result {
            self.expand(s, fs);
            _ = self.format_field(s, key, value, fs, filter);
        }
    }
}

impl RecordWithSourceFormatter for RecordFormatter {
    #[inline]
    fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: model::RecordWithSource) {
        RecordFormatter::format_record(self, buf, prefix_range, rec.record)
    }
}

// ---

struct FormattingStateWithRec<'a> {
    fs: FormattingState<'a>,
    rec: &'a model::Record<'a>,
}

impl<'a> FormattingStateWithRec<'a> {
    fn add_element(&mut self, add_space: impl FnOnce()) {
        if !self.dirty {
            self.dirty = true;
        } else {
            add_space();
        }
    }

    fn transact<R, E, F>(&mut self, s: &mut Styler<Buf>, f: F) -> Result<R, E>
    where
        F: FnOnce(&mut Self, &mut Styler<Buf>) -> Result<R, E>,
    {
        let dirty = self.dirty;
        let depth = self.depth;
        let first_line_used = self.first_line_used;
        let complexity = self.complexity;
        let result = s.transact(|s| f(self, s));
        if result.is_err() {
            self.dirty = dirty;
            self.depth = depth;
            self.first_line_used = first_line_used;
            self.complexity = complexity;
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
    complexity: usize,
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

    fn format<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        key: &str,
        value: RawValue<'a>,
        fs: &mut FormattingStateWithRec,
        filter: Option<&IncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
    ) -> FieldFormatResult {
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

        if !fs.expanded && value.raw_str().len() > fs.expansion.thresholds.field {
            return FieldFormatResult::ExpansionNeeded;
        }

        let ffv = self.begin(s, key, value, fs);

        fs.complexity += key.len() + 2;

        let result = if self.rf.unescape_fields {
            self.format_value(s, value, fs, filter, setting)
        } else {
            s.element(Element::String, |s| {
                s.batch(|buf| buf.extend(value.raw_str().as_bytes()))
            });
            ValueFormatResult::Ok
        };

        self.end(fs, ffv);

        match result {
            ValueFormatResult::Ok => FieldFormatResult::Ok,
            ValueFormatResult::ExpansionNeeded => FieldFormatResult::ExpansionNeeded,
        }
    }

    fn format_value<S: StylingPush<Buf>>(
        &mut self,
        s: &mut S,
        value: RawValue<'a>,
        fs: &mut FormattingStateWithRec,
        filter: Option<&IncludeExcludeKeyFilter>,
        setting: IncludeExcludeSetting,
    ) -> ValueFormatResult {
        let value = match value {
            RawValue::String(EncodedString::Raw(value)) => RawValue::auto(value.as_str()),
            _ => value,
        };

        let complexity_limit = if !fs.expanded {
            Some(std::cmp::min(
                fs.expansion.thresholds.field,
                fs.expansion.thresholds.cumulative - std::cmp::min(fs.expansion.thresholds.cumulative, fs.complexity),
            ))
        } else {
            None
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
                        ValueFormatAuto::default()
                            .with_complexity_limit(complexity_limit)
                            .format(value, buf, xsa)
                            .unwrap()
                    })
                });
                match result {
                    string::FormatResult::Ok(analysis) => {
                        if let Some(analysis) = analysis {
                            fs.complexity += analysis.complexity;
                        }
                    }
                    string::FormatResult::Aborted => {
                        return ValueFormatResult::ExpansionNeeded;
                    }
                }
            }
            RawValue::Number(value) => {
                s.element(Element::Number, |s| s.batch(|buf| buf.extend(value.as_bytes())));
                fs.complexity += value.len();
            }
            RawValue::Boolean(true) => {
                s.element(Element::BooleanTrue, |s| s.batch(|buf| buf.extend(b"true")));
                fs.complexity += 4;
            }
            RawValue::Boolean(false) => {
                s.element(Element::BooleanFalse, |s| s.batch(|buf| buf.extend(b"false")));
                fs.complexity += 5;
            }
            RawValue::Null => {
                s.element(Element::Null, |s| s.batch(|buf| buf.extend(b"null")));
                fs.complexity += 4;
            }
            RawValue::Object(value) => {
                if let Some(limit) = complexity_limit {
                    if !fs.flatten && value.rough_complexity() > limit {
                        return ValueFormatResult::ExpansionNeeded;
                    }
                }

                fs.complexity += 4;
                let item = value.parse().unwrap();
                if !fs.flatten && (!fs.expanded || value.is_empty()) {
                    s.element(Element::Object, |s| {
                        s.batch(|buf| buf.push(b'{'));
                    });
                }
                let mut some_fields_hidden = false;
                for (k, v) in item.fields.iter() {
                    if !self.rf.hide_empty_fields || !v.is_empty() {
                        match self.format(s, k, *v, fs, filter, setting) {
                            FieldFormatResult::Ok => {}
                            FieldFormatResult::Hidden => {
                                some_fields_hidden = true;
                            }
                            FieldFormatResult::ExpansionNeeded => {
                                return ValueFormatResult::ExpansionNeeded;
                            }
                        }
                    }
                }
                if some_fields_hidden {
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
                            if item.fields.len() != 0 {
                                buf.push(b' ');
                            }
                            buf.push(b'}');
                        });
                    });
                }
                fs.some_nested_fields_hidden |= some_fields_hidden;
            }
            RawValue::Array(value) => {
                if let Some(limit) = complexity_limit {
                    if value.rough_complexity() > limit {
                        return ValueFormatResult::ExpansionNeeded;
                    }
                }

                fs.complexity += 4;
                let xb = std::mem::replace(&mut fs.expanded, false);
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
                        _ = self.format_value(s, *v, fs, None, IncludeExcludeSetting::Unspecified);
                    }
                    s.batch(|buf| buf.push(b']'));
                });
                fs.expanded = xb;
            }
        };

        ValueFormatResult::Ok
    }

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

    #[inline]
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
enum ValueFormatResult {
    Ok,
    ExpansionNeeded,
}

#[must_use]
enum FieldFormatResult {
    Ok,
    Hidden,
    ExpansionNeeded,
}

#[must_use]
enum MessageFormatError {
    ExpansionNeeded,
    FormattingAsFieldNeeded,
    EmptyMessage,
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
    use thiserror::Error;

    // workspace imports
    use encstr::{AnyEncodedString, EncodedString, JsonAppender};
    use enumset_ext::EnumSetExt;
    use mline::prefix_lines_within;

    // local imports
    use crate::{
        formatting::WithAutoTrim,
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

    type Result<T> = std::result::Result<T, Error>;

    // ---

    pub trait Format {
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            xsa: ExtendedSpaceAction<'a>,
        ) -> Result<FormatResult>;

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

    pub trait Analyze {
        fn analyze(&self) -> Analysis;
    }

    impl Analyze for [u8] {
        #[inline]
        fn analyze(&self) -> Analysis {
            let mut chars = Mask::empty();
            let mut complexity = 0;
            self.iter()
                .map(|&c| (CHAR_GROUPS[c as usize], COMPLEXITY[c as usize]))
                .for_each(|(group, cc)| {
                    chars |= group;
                    complexity += cc;
                });
            Analysis { chars, complexity }
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

    // impl<'a> ExtendedSpaceAction<'a> {
    //     #[inline]
    //     pub fn map_expand<P2, F>(&self, f: F) -> ExtendedSpaceAction<P2>
    //     where
    //         F: FnOnce(&P) -> P2,
    //     {
    //         match self {
    //             Self::Expand(prefix) => ExtendedSpaceAction::Expand(f(prefix)),
    //             Self::Inline => ExtendedSpaceAction::Inline,
    //             Self::Escape => ExtendedSpaceAction::Escape,
    //             Self::Abort => ExtendedSpaceAction::Abort,
    //         }
    //     }
    // }

    #[must_use]
    pub enum FormatResult {
        Ok(Option<Analysis>),
        Aborted,
    }

    impl FormatResult {
        #[inline]
        #[cfg(test)]
        pub fn is_ok(&self) -> bool {
            matches!(self, Self::Ok(_))
        }
    }

    // ---

    pub struct Analysis {
        pub chars: Mask,
        pub complexity: usize,
    }

    impl Analysis {
        #[inline]
        pub fn empty() -> Self {
            Self {
                chars: Mask::empty(),
                complexity: 2,
            }
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

    #[derive(Default)]
    pub struct ValueFormatAuto {
        complexity_limit: Option<usize>,
    }

    impl ValueFormatAuto {
        #[inline]
        pub fn with_complexity_limit(self, limit: Option<usize>) -> Self {
            Self {
                complexity_limit: limit,
            }
        }
    }

    impl Format for ValueFormatAuto {
        #[inline(always)]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            xsa: ExtendedSpaceAction<'a>,
        ) -> Result<FormatResult> {
            if input.is_empty() {
                buf.extend(r#""""#.as_bytes());
                return Ok(FormatResult::Ok(Some(Analysis::empty())));
            }

            let begin = buf.len();
            _ = buf.with_auto_trim(|buf| ValueFormatRaw.format(input, buf, xsa))?;

            let analysis = buf[begin..].analyze();
            let mask = analysis.chars;

            if let Some(limit) = self.complexity_limit {
                if analysis.complexity > limit {
                    return Ok(FormatResult::Aborted);
                }
            }

            const NON_PLAIN: Mask = mask!(
                Flag::DoubleQuote
                    | Flag::Control
                    | Flag::Backslash
                    | Flag::Space
                    | Flag::EqualSign
                    | Flag::NewLine
                    | Flag::Tab
            );
            const POSITIVE_INTEGER: Mask = mask!(Flag::Digit);
            const NUMBER: Mask = mask!(Flag::Digit | Flag::Dot | Flag::Minus);

            let confusing = || {
                matches!(
                    buf[begin..],
                    [b'{', ..]
                        | [b'[', ..]
                        | [b't', b'r', b'u', b'e']
                        | [b'f', b'a', b'l', b's', b'e']
                        | [b'n', b'u', b'l', b'l']
                )
            };

            let like_number = || {
                (mask == POSITIVE_INTEGER && buf[begin..].len() <= MAX_NUMBER_LEN)
                    || (!mask.intersects(!NUMBER) && looks_like_number(&buf[begin..]))
            };

            if !mask.intersects(NON_PLAIN) && !like_number() && !confusing() {
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            // TODO: validate this block
            // {
            const Z: Mask = Mask::empty();
            const XS: Mask = mask!(Flag::NewLine | Flag::Tab);
            const BT: Mask = mask!(Flag::Backtick);
            const CTL: Mask = mask!(Flag::Control);

            match (mask & CTL, (mask & BT, (mask & XS) != Z), xsa) {
                (Z, (Z, false), _) | (Z, (Z, true), ExtendedSpaceAction::Inline) => {
                    buf.push(b'`');
                    buf.push(b'`');
                    buf[begin..].rotate_right(1);
                    return Ok(FormatResult::Ok(Some(analysis)));
                }
                (Z, _, ExtendedSpaceAction::Expand(prefix)) => {
                    let l0 = buf.len();
                    let pl = prefix(buf);
                    let n = buf.len() - l0;
                    buf[begin..].rotate_right(n);
                    prefix_lines_within(buf, begin + n.., 1.., (begin + n - pl)..(begin + n));
                    return Ok(FormatResult::Ok(Some(analysis)));
                }
                (Z, _, ExtendedSpaceAction::Abort) => {
                    buf.truncate(begin);
                    return Ok(FormatResult::Aborted);
                }
                _ => {
                    buf.truncate(begin);
                    ValueFormatDoubleQuoted.format(input, buf, xsa)
                }
            }
            // }

            // if !mask.intersects(Flag::Backtick | Flag::Control) {
            //     buf.push(b'`');
            //     buf.push(b'`');
            //     buf[begin..].rotate_right(1);
            //     return Ok(());
            // }

            // buf.truncate(begin);
            // ValueFormatDoubleQuoted.format(input, buf)
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
            _: ExtendedSpaceAction<'a>,
        ) -> Result<FormatResult> {
            input.decode(buf)?;
            Ok(FormatResult::Ok(None))
        }
    }

    // ---

    pub struct ValueFormatDoubleQuoted;

    impl Format for ValueFormatDoubleQuoted {
        #[inline]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            _: ExtendedSpaceAction<'a>,
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
            xsa: ExtendedSpaceAction<'a>,
        ) -> Result<FormatResult> {
            if input.is_empty() {
                buf.extend(r#""""#.as_bytes());
                return Ok(FormatResult::Ok(Some(Analysis::empty())));
            }

            let begin = buf.len();
            _ = buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf, xsa))?;

            let analysis = buf[begin..].analyze();
            let mask = analysis.chars;

            const NON_PLAIN: Mask = mask!(
                Flag::EqualSign
                    | Flag::Control
                    | Flag::NewLine
                    | Flag::Backslash
                    | Flag::Colon
                    | Flag::Tilde
                    | Flag::AngleBrackets
                    | Flag::DoubleQuote
                    | Flag::SingleQuote
                    | Flag::Backtick
            );

            if !mask.intersects(NON_PLAIN) {
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            // if !mask.intersects(Flag::EqualSign | Flag::Control | Flag::NewLine | Flag::Backslash)
            //     && !matches!(buf[begin..], [b'"', ..] | [b'\'', ..] | [b'`', ..])
            // {
            //     return Ok(());
            // }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            const Z: Mask = Mask::empty();
            const XS: Mask = mask!(Flag::NewLine | Flag::Tab);
            const BT: Mask = mask!(Flag::Backtick);
            const CTL: Mask = mask!(Flag::Control);

            match (mask & CTL, (mask & BT, (mask & XS) != Z), xsa) {
                (Z, (Z, false), _) | (Z, (Z, true), ExtendedSpaceAction::Inline) => {
                    buf.push(b'`');
                    buf.push(b'`');
                    buf[begin..].rotate_right(1);
                    return Ok(FormatResult::Ok(Some(analysis)));
                }
                (Z, _, ExtendedSpaceAction::Abort) => {
                    buf.truncate(begin);
                    Ok(FormatResult::Aborted)
                }
                _ => {
                    buf.truncate(begin);
                    MessageFormatDoubleQuoted.format(input, buf, xsa)
                }
            }

            // if !mask.intersects(Flag::Backtick | Flag::Control) {
            //     buf.push(b'`');
            //     buf.push(b'`');
            //     buf[begin..].rotate_right(1);
            //     return Ok(FormatResult::Ok(Some(analysis)));
            // }

            // buf.truncate(begin);
            // MessageFormatDoubleQuoted.format(input, buf)
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
            xsa: ExtendedSpaceAction<'a>,
        ) -> Result<FormatResult> {
            if input.is_empty() {
                return Ok(FormatResult::Ok(Some(Analysis::empty())));
            }

            let begin = buf.len();
            buf.push(b'"');
            _ = buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf, xsa))?;

            let analysis = buf[begin + 1..].analyze();
            let mask = analysis.chars;

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf.push(b'"');
                return Ok(FormatResult::Ok(None));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::NewLine | Flag::Backslash) {
                buf[begin] = b'\'';
                buf.push(b'\'');
                return Ok(FormatResult::Ok(None));
            }

            if !mask.intersects(Flag::Backtick | Flag::Control) {
                buf[begin] = b'`';
                buf.push(b'`');
                return Ok(FormatResult::Ok(None));
            }

            buf.truncate(begin);
            MessageFormatDoubleQuoted.format(input, buf, xsa)
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
            xsa: ExtendedSpaceAction<'a>,
        ) -> Result<FormatResult> {
            if input.is_empty() {
                return Ok(FormatResult::Ok(None));
            }

            let begin = buf.len();
            _ = buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf, xsa))?;

            let analysis = buf[begin..].analyze();
            let mask = analysis.chars;

            if !mask.contains(Flag::Control)
                && !matches!(buf[begin..], [b'"', ..] | [b'\'', ..] | [b'`', ..])
                && memchr::memmem::find(&buf[begin..], self.0.as_bytes()).is_none()
            {
                buf.extend(self.0.as_bytes());
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                buf.extend(self.0.as_bytes());
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                buf.extend(self.0.as_bytes());
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::Backtick | Flag::Control) {
                buf.push(b'`');
                buf.push(b'`');
                buf[begin..].rotate_right(1);
                buf.extend(self.0.as_bytes());
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            buf.truncate(begin);
            let result = MessageFormatDoubleQuoted.format(input, buf, xsa)?;
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
            _: ExtendedSpaceAction<'a>,
        ) -> Result<FormatResult> {
            input.decode(buf)?;
            Ok(FormatResult::Ok(None))
        }
    }

    // ---

    pub struct MessageFormatDoubleQuoted;

    impl Format for MessageFormatDoubleQuoted {
        #[inline]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            _: ExtendedSpaceAction<'a>,
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
        fn new(n: usize, inner: F) -> Self {
            Self { n, inner }
        }
    }

    impl<F: Format> Format for FormatRightTrimmed<F> {
        #[inline]
        fn format<'a>(
            &self,
            input: EncodedString<'a>,
            buf: &mut Vec<u8>,
            xsa: ExtendedSpaceAction<'a>,
        ) -> Result<FormatResult> {
            let begin = buf.len();
            let result = self.inner.format(input, buf, xsa)?;
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

    static COMPLEXITY: [usize; 256] = {
        const XS: usize = 32;
        const CT: usize = 8;
        const QU: usize = 4;
        const EQ: usize = 4;
        const BS: usize = 8;
        const __: usize = 1;
        [
            //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
            CT, CT, CT, CT, CT, CT, CT, CT, CT, XS, XS, CT, CT, XS, CT, CT, // 0
            CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 1
            __, __, QU, __, __, __, __, QU, __, __, __, __, __, __, __, __, // 2
            __, __, __, __, __, __, __, __, __, __, __, __, __, EQ, __, __, // 3
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
            __, __, __, __, __, __, __, __, __, __, __, __, BS, __, __, __, // 5
            __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
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
    pub enum Flag {
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
