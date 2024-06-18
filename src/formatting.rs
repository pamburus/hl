// std imports
use std::{
    ops::{Deref, DerefMut, Range},
    sync::Arc,
};

// workspace imports
use encstr::EncodedString;

// local imports
use crate::{
    datefmt::{DateTimeFormatter, TextWidth},
    filtering::IncludeExcludeSetting,
    fmtx::{aligned_left, centered, OptimizedBuf, Push},
    model::{self, Caller, Level, RawValue},
    settings::{self, ExpandOption, FlattenOption, Formatting, Punctuation},
    syntax::*,
    theme::{Element, Styler, StylingPush, Theme},
    IncludeExcludeKeyFilter,
};

// relative imports
use string::{ExtendedSpaceAction, Format, MessageFormatAuto, ValueFormatAuto};

// ---

const DEFAULT_EXPAND_ALL_THRESHOLD: usize = 1024;
const DEFAULT_EXPAND_CUMULATIVE_THRESHOLD: usize = 256;
const DEFAULT_EXPAND_MESSAGE_THRESHOLD: usize = 192;
const DEFAULT_EXPAND_FIELD_THRESHOLD: usize = 64;

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

pub struct Expansion {
    pub mode: ExpandOption,
    pub thresholds: ExpansionThresholds,
}

impl Expansion {
    pub fn with_mode(mut self, mode: ExpandOption) -> Self {
        self.mode = mode;
        self
    }
}

impl Default for Expansion {
    fn default() -> Self {
        Self {
            mode: ExpandOption::Auto,
            thresholds: Default::default(),
        }
    }
}

impl From<ExpandOption> for Expansion {
    fn from(mode: ExpandOption) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }
}

impl From<settings::ExpansionOptions> for Expansion {
    fn from(options: settings::ExpansionOptions) -> Self {
        Self {
            mode: options.mode.unwrap_or_default(),
            thresholds: options.thresholds.into(),
        }
    }
}

// ---

pub struct ExpansionThresholds {
    pub global: usize,
    pub cumulative: usize,
    pub message: usize,
    pub field: usize,
}

impl Default for ExpansionThresholds {
    fn default() -> Self {
        Self {
            global: DEFAULT_EXPAND_ALL_THRESHOLD,
            cumulative: DEFAULT_EXPAND_CUMULATIVE_THRESHOLD,
            message: DEFAULT_EXPAND_MESSAGE_THRESHOLD,
            field: DEFAULT_EXPAND_FIELD_THRESHOLD,
        }
    }
}

impl From<settings::ExpansionThresholds> for ExpansionThresholds {
    fn from(options: settings::ExpansionThresholds) -> Self {
        Self {
            global: options.global.unwrap_or(DEFAULT_EXPAND_ALL_THRESHOLD),
            cumulative: options.cumulative.unwrap_or(DEFAULT_EXPAND_CUMULATIVE_THRESHOLD),
            message: options.message.unwrap_or(DEFAULT_EXPAND_MESSAGE_THRESHOLD),
            field: options.field.unwrap_or(DEFAULT_EXPAND_FIELD_THRESHOLD),
        }
    }
}

// ---

pub struct RecordFormatterSettings {
    pub theme: Arc<Theme>,
    pub unescape_fields: bool,
    pub ts_formatter: DateTimeFormatter,
    pub hide_empty_fields: bool,
    pub flatten: bool,
    pub always_show_time: bool,
    pub always_show_level: bool,
    pub expansion: Expansion,
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
            always_show_time: false,
            always_show_level: false,
            expansion: cfg.expansion.into(),
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
    cfg: RecordFormatterSettings,
    ts_width: TextWidth,
}

impl RecordFormatter {
    pub fn new(cfg: RecordFormatterSettings) -> Self {
        let ts_width = cfg.ts_formatter.max_width();
        Self { cfg, ts_width }
    }

    pub fn format_record(&self, buf: &mut Buf, prefix_range: Range<usize>, rec: &model::Record) {
        let mut fs = FormattingStateWithRec {
            rec,
            fs: FormattingState {
                flatten: self.cfg.flatten && self.cfg.unescape_fields,
                expand: self.cfg.expansion.mode.into(),
                prefix: prefix_range,
                ..Default::default()
            },
        };

        self.cfg.theme.apply(buf, &rec.level, |s| {
            //
            // time
            //
            if fs.transact(s, |fs, s| self.format_timestamp(rec, fs, s)).is_err() {
                if let Some(ts) = &rec.ts {
                    fs.extra_fields
                        .push(("ts", RawValue::String(EncodedString::raw(ts.raw()))))
                        .ok();
                }
                if self.cfg.always_show_time {
                    self.format_timestamp_stub(&mut fs, s);
                }
            }

            //
            // level
            //
            let level = match rec.level {
                Some(Level::Debug) => Some(LEVEL_DEBUG.as_bytes()),
                Some(Level::Info) => Some(LEVEL_INFO.as_bytes()),
                Some(Level::Warning) => Some(LEVEL_WARNING.as_bytes()),
                Some(Level::Error) => Some(LEVEL_ERROR.as_bytes()),
                None => None,
            };
            let level = level.or_else(|| self.cfg.always_show_level.then(|| LEVEL_UNKNOWN.as_bytes()));
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
                    s.batch(|buf| buf.extend_from_slice(self.cfg.punctuation.logger_name_separator.as_bytes()));
                    fs.complexity += logger.len() + 4;
                    fs.first_line_used = true;
                });
            }
            //
            // message text
            //
            if let Some(value) = &rec.message {
                match fs.transact(s, |fs, s| self.format_message(s, fs, *value)) {
                    Ok(()) => {
                        fs.complexity += 4;
                        fs.first_line_used = true;
                    }
                    Err(MessageFormatError::ExpansionNeeded) => {
                        self.add_field_to_expand(s, &mut fs, "msg", *value, Some(&self.cfg.fields));
                    }
                    Err(MessageFormatError::FormattingAsFieldNeeded) => {
                        fs.extra_fields.push(("msg", *value)).ok();
                    }
                }
            } else {
                s.reset();
            }

            fs.expand = fs.expand.or_else(|| {
                let thresholds = &self.cfg.expansion.thresholds;
                if fs.complexity >= thresholds.cumulative {
                    Some(true)
                } else if self.complexity(fs.complexity, rec, Some(&self.cfg.fields)) >= thresholds.global {
                    Some(true)
                } else {
                    None
                }
            });

            //
            // fields
            //
            let mut some_fields_hidden = false;
            let x_fields = std::mem::take(&mut fs.extra_fields);
            for (k, v) in x_fields.iter().chain(rec.fields()) {
                if !self.cfg.hide_empty_fields || !v.is_empty() {
                    let result = fs.transact(s, |fs, s| {
                        match self.format_field(s, k, *v, fs, Some(&self.cfg.fields)) {
                            FieldFormatResult::Ok => {
                                if fs.expand != Some(true) {
                                    fs.first_line_used = true;
                                }
                                Ok(())
                            }
                            FieldFormatResult::Hidden => {
                                some_fields_hidden = true;
                                Ok(())
                            }
                            FieldFormatResult::ExpansionNeeded => Err(()),
                        }
                    });
                    if let Err(()) = result {
                        self.add_field_to_expand(s, &mut fs, k, *v, Some(&self.cfg.fields));
                    }
                }
            }

            //
            // expanded fields
            //
            if fs.fields_to_expand.len() != 0 {
                self.expand(s, &mut fs);
            }

            if some_fields_hidden || fs.some_fields_hidden {
                if fs.expand == Some(true) {
                    self.expand(s, &mut fs);
                }
                fs.add_element(|| s.batch(|buf| buf.push(b' ')));
                s.element(Element::Ellipsis, |s| {
                    s.batch(|buf| buf.extend_from_slice(self.cfg.punctuation.hidden_fields_indicator.as_bytes()))
                });
            }

            //
            // caller
            //
            if !fs.caller_formatted {
                if let Some(caller) = &rec.caller {
                    self.format_caller(s, caller);
                };
            }
        });
    }

    #[inline]
    #[must_use]
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
                        .and_then(|ts| self.cfg.ts_formatter.reformat_rfc3339(&mut buf, ts))
                        .is_some()
                    {
                        Ok(())
                    } else if let Some(ts) = ts.parse() {
                        self.cfg.ts_formatter.format(&mut buf, ts);
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
    fn complexity(&self, initial: usize, rec: &model::Record, filter: Option<&IncludeExcludeKeyFilter>) -> usize {
        let mut result = initial;
        result += rec.message.map(|x| x.raw_str().len() / 8).unwrap_or(0);
        result += rec.predefined.len();
        result += rec.logger.map(|x| x.len() / 2).unwrap_or(0);
        for (key, value) in rec.fields() {
            if value.is_empty() {
                if self.cfg.hide_empty_fields {
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
    fn format_caller<S: StylingPush<Buf>>(&self, s: &mut S, caller: &Caller) {
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
    #[must_use]
    fn format_message<'a, S: StylingPush<Buf>>(
        &self,
        s: &mut S,
        fs: &mut FormattingStateWithRec,
        value: RawValue<'a>,
    ) -> Result<(), MessageFormatError> {
        match value {
            RawValue::String(value) => {
                if !value.is_empty() {
                    if value.source().len() > self.cfg.expansion.thresholds.message {
                        fs.expand = Some(fs.expand.unwrap_or(true));
                    }
                    fs.add_element(|| {
                        s.reset();
                        s.space();
                    });
                    s.element(Element::Message, |s| {
                        s.batch(|buf| {
                            let result = MessageFormatAuto::new(value)
                                .on_extended_space::<()>(if fs.expand.unwrap_or(true) {
                                    ExtendedSpaceAction::Abort
                                } else {
                                    ExtendedSpaceAction::FormatWithBacktick
                                })
                                .format(buf)
                                .unwrap();
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
                    Ok(())
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
                buf.extend_from_slice(self.cfg.punctuation.level_left_separator.as_bytes());
            });
            s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(level)));
            s.batch(|buf| buf.extend_from_slice(self.cfg.punctuation.level_right_separator.as_bytes()));
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
            if let Some(caller) = &fs.rec.caller {
                self.format_caller(s, caller);
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

        if fs.expand != Some(true) {
            fs.expand = Some(true);
            let fields_to_expand = std::mem::take(&mut fs.fields_to_expand);
            for (k, v) in fields_to_expand.iter() {
                _ = self.format_field(s, k, *v, fs, None);
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
        let result = if fs.expand == Some(true) {
            Err((key, value))
        } else {
            fs.fields_to_expand.push((key, value))
        };

        if let Err((key, value)) = result {
            fs.expand = Some(true);
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

impl From<RecordFormatterSettings> for RecordFormatter {
    #[inline]
    fn from(settings: RecordFormatterSettings) -> Self {
        RecordFormatter::new(settings)
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
    flatten: bool,
    expand: Option<bool>,
    prefix: Range<usize>,
    expansion_prefix: Option<Range<usize>>,
    key_prefix: KeyPrefix,
    dirty: bool,
    ts_width: usize,
    has_level: bool,
    depth: usize,
    first_line_used: bool,
    some_fields_hidden: bool,
    caller_formatted: bool,
    complexity: usize,
    extra_fields: heapless::Vec<(&'a str, RawValue<'a>), 4>,
    fields_to_expand: heapless::Vec<(&'a str, RawValue<'a>), 32>,
    last_expansion_point: Option<usize>,
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

        if fs.expand.is_none() && value.raw_str().len() > self.rf.cfg.expansion.thresholds.field {
            return FieldFormatResult::ExpansionNeeded;
        }

        let ffv = self.begin(s, key, value, fs);

        fs.complexity += key.len() + 4;

        let result = if self.rf.cfg.unescape_fields {
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

        let complexity_limit = if fs.expand.is_none() {
            Some(std::cmp::min(
                self.rf.cfg.expansion.thresholds.field,
                self.rf.cfg.expansion.thresholds.cumulative
                    - std::cmp::min(self.rf.cfg.expansion.thresholds.cumulative, fs.complexity),
            ))
        } else {
            None
        };

        match value {
            RawValue::String(value) => {
                let result = s.element(Element::String, |s| {
                    s.batch(|buf| {
                        ValueFormatAuto::new(value)
                            .on_extended_space(match fs.expand {
                                Some(true) => ExtendedSpaceAction::Expand(|buf: &mut Vec<u8>| self.add_prefix(buf, fs)),
                                Some(false) => ExtendedSpaceAction::FormatWithBacktick,
                                None => ExtendedSpaceAction::Abort,
                            })
                            .with_complexity_limit(complexity_limit)
                            .format(buf)
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
                s.element(Element::Boolean, |s| s.batch(|buf| buf.extend(b"true")));
                fs.complexity += 4;
            }
            RawValue::Boolean(false) => {
                s.element(Element::Boolean, |s| s.batch(|buf| buf.extend(b"false")));
                fs.complexity += 5;
            }
            RawValue::Null => {
                s.element(Element::Null, |s| s.batch(|buf| buf.extend(b"null")));
                fs.complexity += 4;
            }
            RawValue::Object(value) => {
                const FIXED_COMPLEXITY: usize = 4;

                if let Some(limit) = complexity_limit {
                    if FIXED_COMPLEXITY + value.get().len() > limit {
                        return ValueFormatResult::ExpansionNeeded;
                    }
                }

                fs.complexity += FIXED_COMPLEXITY;
                let item = value.parse().unwrap();
                if !fs.flatten && (!fs.expand.unwrap_or(false) || value.is_empty()) {
                    s.element(Element::Object, |s| {
                        s.batch(|buf| buf.push(b'{'));
                    });
                }
                let mut some_fields_hidden = false;
                for (k, v) in item.fields.iter() {
                    if !self.rf.cfg.hide_empty_fields || !v.is_empty() {
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
                        if fs.expand.unwrap_or(false) {
                            self.rf.expand(s, fs);
                        }
                        s.element(Element::Ellipsis, |s| {
                            s.batch(|buf| buf.extend(self.rf.cfg.punctuation.hidden_fields_indicator.as_bytes()))
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
                const FIXED_COMPLEXITY: usize = 4;

                if let Some(limit) = complexity_limit {
                    if FIXED_COMPLEXITY + value.get().len() > limit {
                        return ValueFormatResult::ExpansionNeeded;
                    }
                }

                fs.complexity += FIXED_COMPLEXITY;
                let xb = fs.expand.replace(false);
                let item = value.parse::<32>().unwrap();
                s.element(Element::Array, |s| {
                    s.batch(|buf| buf.push(b'['));
                });
                let mut first = true;
                for v in item.iter() {
                    if !first {
                        s.batch(|buf| buf.extend(self.rf.cfg.punctuation.array_separator.as_bytes()));
                    } else {
                        first = false;
                    }
                    _ = self.format_value(s, *v, fs, None, IncludeExcludeSetting::Unspecified);
                }
                s.element(Element::Array, |s| {
                    s.batch(|buf| buf.push(b']'));
                });
                fs.expand = xb;
            }
        };

        ValueFormatResult::Ok
    }

    fn add_prefix(&self, buf: &mut Vec<u8>, fs: &FormattingStateWithRec) -> usize {
        buf.extend(self.rf.cfg.theme.expanded_value_suffix.value.as_bytes());
        buf.push(b'\n');
        let prefix = fs.expansion_prefix.clone().unwrap_or(fs.prefix.clone());
        let l0 = buf.len();
        buf.extend_from_within(prefix);
        buf.extend(self.rf.cfg.theme.expanded_value_prefix.value.as_bytes());
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
            EXPANDED_OBJECT_HEADER.as_bytes()
        } else {
            self.rf.cfg.punctuation.field_key_value_separator.as_bytes()
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
enum MessageFormatError {
    ExpansionNeeded,
    FormattingAsFieldNeeded,
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
    }

    // ---

    type Result<T> = std::result::Result<T, Error>;

    // ---

    pub trait Format {
        fn format(&self, buf: &mut Vec<u8>) -> Result<FormatResult>;
    }

    pub trait Analyze {
        fn analyze(&self) -> Analysis;
    }

    impl Analyze for [u8] {
        #[inline]
        fn analyze(&self) -> Analysis {
            let mut chars = Mask::EMPTY;
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

    pub enum ExtendedSpaceAction<P = ()> {
        FormatWithBacktick,
        Expand(P),
        Escape,
        Abort,
    }

    impl<P> ExtendedSpaceAction<P> {
        #[inline]
        pub fn map_expand<P2, F>(&self, f: F) -> ExtendedSpaceAction<P2>
        where
            F: FnOnce(&P) -> P2,
        {
            match self {
                Self::Expand(prefix) => ExtendedSpaceAction::Expand(f(prefix)),
                Self::FormatWithBacktick => ExtendedSpaceAction::FormatWithBacktick,
                Self::Escape => ExtendedSpaceAction::Escape,
                Self::Abort => ExtendedSpaceAction::Abort,
            }
        }
    }

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
                chars: Mask::EMPTY,
                complexity: 2,
            }
        }
    }

    // ---

    pub struct ValueFormatAuto<S, P = ()> {
        string: S,
        xs_action: ExtendedSpaceAction<P>,
        complexity_limit: Option<usize>,
    }

    impl<S> ValueFormatAuto<S, ()> {
        #[inline]
        pub fn new(string: S) -> Self {
            Self {
                string,
                xs_action: ExtendedSpaceAction::FormatWithBacktick,
                complexity_limit: None,
            }
        }

        #[inline]
        pub fn on_extended_space<P>(self, action: ExtendedSpaceAction<P>) -> ValueFormatAuto<S, P> {
            ValueFormatAuto::<S, P> {
                xs_action: action,
                string: self.string,
                complexity_limit: self.complexity_limit,
            }
        }
    }

    impl<S, P> ValueFormatAuto<S, P> {
        #[inline]
        pub fn with_complexity_limit(self, limit: Option<usize>) -> Self {
            Self {
                complexity_limit: limit,
                ..self
            }
        }
    }

    impl<'a, S> Format for ValueFormatAuto<S, ()>
    where
        S: AnyEncodedString<'a> + Clone + Copy,
    {
        #[inline]
        fn format(&self, buf: &mut Vec<u8>) -> Result<FormatResult> {
            ValueFormatAuto {
                string: self.string,
                xs_action: self.xs_action.map_expand(|_| |_: &mut Vec<u8>| 0),
                complexity_limit: self.complexity_limit,
            }
            .format(buf)
        }
    }

    impl<'a, S, P> Format for ValueFormatAuto<S, P>
    where
        S: AnyEncodedString<'a> + Clone + Copy,
        P: Fn(&mut Vec<u8>) -> usize,
    {
        #[inline]
        fn format(&self, buf: &mut Vec<u8>) -> Result<FormatResult> {
            if self.string.is_empty() {
                buf.extend(r#""""#.as_bytes());
                return Ok(FormatResult::Ok(Some(Analysis::empty())));
            }

            let begin = buf.len();
            _ = buf.with_auto_trim(|buf| ValueFormatRaw::new(self.string).format(buf))?;

            let analysis = buf[begin..].analyze();
            let mask = analysis.chars;

            if let Some(limit) = self.complexity_limit {
                if analysis.complexity > limit {
                    return Ok(FormatResult::Aborted);
                }
            }

            const NON_PLAIN: Mask =
                mask!(Flag::DoubleQuote | Flag::Control | Flag::Backslash | Flag::Space | Flag::EqualSign);
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

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            const Z: Mask = Mask::EMPTY;
            const XS: Mask = mask!(Flag::Control | Flag::ExtendedSpace);
            const BTXS: Mask = mask!(Flag::Control | Flag::ExtendedSpace | Flag::Backtick);

            match (mask & BTXS, &self.xs_action) {
                (Z, _) | (XS, ExtendedSpaceAction::FormatWithBacktick) => {
                    buf.push(b'`');
                    buf.push(b'`');
                    buf[begin..].rotate_right(1);
                    return Ok(FormatResult::Ok(Some(analysis)));
                }
                (XS | BTXS, ExtendedSpaceAction::Expand(prefix)) => {
                    let l0 = buf.len();
                    let pl = prefix(buf);
                    let n = buf.len() - l0;
                    buf[begin..].rotate_right(n);
                    prefix_lines_within(buf, begin + n.., 1.., (begin + n - pl)..(begin + n));
                    return Ok(FormatResult::Ok(Some(analysis)));
                }
                (XS | BTXS, ExtendedSpaceAction::Abort) => {
                    buf.truncate(begin);
                    return Ok(FormatResult::Aborted);
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
        fn format(&self, buf: &mut Vec<u8>) -> Result<FormatResult> {
            self.string.decode(buf)?;
            Ok(FormatResult::Ok(None))
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
        fn format(&self, buf: &mut Vec<u8>) -> Result<FormatResult> {
            self.string.format_json(buf)?;
            Ok(FormatResult::Ok(None))
        }
    }

    // ---

    pub struct MessageFormatAuto<S, P = ()> {
        string: S,
        xs_action: ExtendedSpaceAction<P>,
    }

    impl<S> MessageFormatAuto<S, ()> {
        #[inline]
        pub fn new(string: S) -> Self {
            Self {
                string,
                xs_action: ExtendedSpaceAction::Escape,
            }
        }

        #[inline]
        pub fn on_extended_space<P>(self, action: ExtendedSpaceAction<P>) -> MessageFormatAuto<S, P> {
            MessageFormatAuto::<S, P> {
                xs_action: action,
                string: self.string,
            }
        }
    }

    impl<'a, S> Format for MessageFormatAuto<S, ()>
    where
        S: AnyEncodedString<'a> + Clone + Copy,
    {
        #[inline]
        fn format(&self, buf: &mut Vec<u8>) -> Result<FormatResult> {
            MessageFormatAuto {
                string: self.string,
                xs_action: self.xs_action.map_expand(|_| |_: &mut Vec<u8>| 0),
            }
            .format(buf)
        }
    }

    impl<'a, S, P> Format for MessageFormatAuto<S, P>
    where
        S: AnyEncodedString<'a> + Clone + Copy,
        P: Fn(&mut Vec<u8>) -> usize,
    {
        #[inline]
        fn format(&self, buf: &mut Vec<u8>) -> Result<FormatResult> {
            if self.string.is_empty() {
                buf.extend(r#""""#.as_bytes());
                return Ok(FormatResult::Ok(Some(Analysis::empty())));
            }

            let begin = buf.len();
            _ = buf.with_auto_trim(|buf| MessageFormatRaw::new(self.string).format(buf))?;

            let analysis = buf[begin..].analyze();
            let mask = analysis.chars;

            const NON_PLAIN: Mask = mask!(
                Flag::EqualSign
                    | Flag::Control
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

            if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'"');
                buf.push(b'"');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Backslash) {
                buf.push(b'\'');
                buf.push(b'\'');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            const Z: Mask = Mask::EMPTY;
            const XS: Mask = mask!(Flag::Control | Flag::ExtendedSpace);
            const BTXS: Mask = mask!(Flag::Control | Flag::ExtendedSpace | Flag::Backtick);

            match (mask & BTXS, &self.xs_action) {
                (Z, _) | (XS, ExtendedSpaceAction::FormatWithBacktick) => {
                    buf.push(b'`');
                    buf.push(b'`');
                    buf[begin..].rotate_right(1);
                    return Ok(FormatResult::Ok(Some(analysis)));
                }
                (XS | BTXS, ExtendedSpaceAction::Abort) => {
                    buf.truncate(begin);
                    Ok(FormatResult::Aborted)
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
        fn format(&self, buf: &mut Vec<u8>) -> Result<FormatResult> {
            self.string.decode(buf)?;
            Ok(FormatResult::Ok(None))
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
        fn format(&self, buf: &mut Vec<u8>) -> Result<FormatResult> {
            self.string.format_json(buf)?;
            Ok(FormatResult::Ok(None))
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
        const DT: Mask = mask!(Flag::Dot); // Dot, 0x2E
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
            CT, CT, CT, CT, CT, CT, CT, CT, CT, XS, XS, CT, CT, XS, CT, CT, // 0
            CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 1
            SP, __, DQ, __, __, __, __, SQ, __, __, __, __, __, HY, DT, __, // 2
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
        ExtendedSpace,
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
                LinuxDateFormat::new("%Y-%m-%d %T.%3N").compile(),
                Tz::FixedOffset(Utc.fix()),
            ),
            always_show_time: false,
            punctuation: Arc::new(Punctuation {
                logger_name_separator: ":".into(),
                field_key_value_separator: "=".into(),
                string_opening_quote: "'".into(),
                string_closing_quote: "'".into(),
                source_location_separator: "@ ".into(),
                hidden_fields_indicator: "...".into(),
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
            "\u{1b}[0;2;3m2000-01-02 03:04:05.123 \u{1b}[0;36m|\u{1b}[0;95mDBG\u{1b}[0;36m|\u{1b}[0;2;3m \u{1b}[0;2;4mtl:\u{1b}[0m \u{1b}[0;1;39mtm \u{1b}[0;32mk-a\u{1b}[0;2m=\u{1b}[0;33m{ \u{1b}[0;32mva\u{1b}[0;2m=\u{1b}[0;33m{ \u{1b}[0;32mkb\u{1b}[0;2m=\u{1b}[0;94m42 \u{1b}[0;32mkc\u{1b}[0;2m=\u{1b}[0;94m43\u{1b}[0;33m } }\u{1b}[0;2;3m @ tc\u{1b}[0m",
        );

        let formatter = RecordFormatter::new(settings().with(|s| s.flatten = true));
        assert_eq!(
            formatter.format_to_string(&rec),
            "\u{1b}[0;2;3m2000-01-02 03:04:05.123 \u{1b}[0;36m|\u{1b}[0;95mDBG\u{1b}[0;36m|\u{1b}[0;2;3m \u{1b}[0;2;4mtl:\u{1b}[0m \u{1b}[0;1;39mtm \u{1b}[0;32mk-a.va.kb\u{1b}[0;2m=\u{1b}[0;94m42 \u{1b}[0;32mk-a.va.kc\u{1b}[0;2m=\u{1b}[0;94m43\u{1b}[0;2;3m @ tc\u{1b}[0m",
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
            "\u{1b}[0;2;3m-----------------------\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m",
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
        assert_eq!(
            format_no_color(&rec),
            format!(
                "{mh}\n  > k={vh}\n    {vi}some\tvalue",
                mh = EXPANDED_MESSAGE_HEADER,
                vh = EXPANDED_VALUE_HEADER,
                vi = EXPANDED_VALUE_INDENT,
            )
        );
    }

    #[test]
    fn test_string_value_raw_extended_space() {
        let v = "some\tvalue";
        let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
        assert_eq!(
            format_no_color(&rec),
            format!(
                "{mh}\n  > k={vh}\n    {vi}some\tvalue",
                mh = EXPANDED_MESSAGE_HEADER,
                vh = EXPANDED_VALUE_HEADER,
                vi = EXPANDED_VALUE_INDENT,
            )
        );
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
            fields.entry("c").entry("z").exclude();
            s.theme = Default::default();
            s.flatten = false;
            s.expansion.mode = ExpandOption::Always;
            s.fields = fields.into();
        }));

        let source = br#"{"msg":"m","a":1,"b":2,"c:{"x":10,"y":20,"z":30},"d":4}"#;
        let obj = json_raw_value(r#"{"x":10,"y":20,"z":30}"#);
        let rec = Record {
            message: Some(EncodedString::raw("m").into()),
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[
                    ("a", EncodedString::raw("1").into()),
                    ("b", EncodedString::raw("2").into()),
                    ("c", RawObject::Json(&obj).into()),
                    ("d", EncodedString::raw("4").into()),
                ])
                .unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = formatter.format_to_string(rec.with_source(source));
        assert_eq!(
            &result,
            "m\n  > a=1\n  > c:\n    > x=10\n    > y=20\n    > ...\n  > d=4\n  > ..."
        );
    }

    #[test]
    fn test_expand_with_hidden_and_flatten() {
        let formatter = RecordFormatter::new(settings().with(|s| {
            let mut fields = IncludeExcludeKeyFilter::default();
            fields.entry("c").entry("z").exclude();
            s.theme = Default::default();
            s.flatten = true;
            s.expansion.mode = ExpandOption::Always;
            s.fields = fields.into();
        }));

        let obj = json_raw_value(r#"{"x":10,"y":20,"z":30}"#);
        let rec = Record {
            message: Some(EncodedString::raw("m").into()),
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[
                    ("a", EncodedString::raw("1").into()),
                    ("b", EncodedString::raw("2").into()),
                    ("c", RawObject::Json(&obj).into()),
                    ("d", EncodedString::raw("4").into()),
                ])
                .unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = formatter.format_to_string(&rec);
        assert_eq!(&result, "m\n  > a=1\n  > b=2\n  > c.x=10\n  > c.y=20\n  > d=4\n  > ...");
    }

    #[test]
    fn test_expand_object() {
        let formatter = RecordFormatter::new(settings().with(|s| {
            s.theme = Default::default();
            s.flatten = false;
            s.expansion.mode = ExpandOption::Auto;
        }));

        let obj = json_raw_value(r#"{"x":10,"y":"some\nmultiline\nvalue","z":30}"#);
        let rec = Record {
            message: Some(EncodedString::raw("m").into()),
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[
                    ("a", EncodedString::raw("1").into()),
                    ("b", EncodedString::raw("2").into()),
                    ("c", RawObject::Json(&obj).into()),
                    ("d", EncodedString::raw("4").into()),
                ])
                .unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = formatter.format_to_string(&rec);
        assert_eq!(
            &result,
            "m a=1 b=2 d=4\n  > c:\n    > x=10\n    > y=|=>\n       \tsome\n       \tmultiline\n       \tvalue\n    > z=30"
        );
    }

    #[test]
    fn test_expand_all_threshold() {
        let formatter = RecordFormatter::new(settings().with(|s| {
            s.theme = Default::default();
            s.expansion.mode = ExpandOption::Auto;
            s.expansion.thresholds.global = 2;
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
        assert_eq!(&result, "m\n  > a=1\n  > b=2\n  > c=3", "{}", result);
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

    #[test]
    fn test_expand_no_filter() {
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

        let formatter = RecordFormatter::new(settings().with(|s| {
            s.theme = Default::default();
            s.expansion.mode = ExpandOption::Auto;
        }));

        assert_eq!(formatter.format_to_string(&rec), r#"m a=1 b=2 c=3"#);
    }

    #[test]
    fn test_expand_message() {
        let rec = |m, f| Record {
            message: Some(EncodedString::raw(m).into()),
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[("a", EncodedString::raw(f).into())]).unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut formatter = RecordFormatter::new(settings().with(|s| {
            s.theme = Default::default();
            s.expansion.mode = ExpandOption::Auto;
            s.expansion.thresholds.message = 64;
        }));

        let lorem_ipsum = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";

        assert_eq!(
            formatter.format_to_string(&rec(lorem_ipsum, "1")),
            lorem_ipsum.to_owned() + "\n  > a=1"
        );
        assert_eq!(
            formatter.format_to_string(&rec("", "some\nmultiline\ntext")),
            format!(
                concat!(
                    "\n",
                    "  > a={header}\n",
                    "    {indent}some\n",
                    "    {indent}multiline\n",
                    "    {indent}text"
                ),
                header = EXPANDED_VALUE_HEADER,
                indent = EXPANDED_VALUE_INDENT
            )
        );

        assert_eq!(
            formatter.format_to_string(&rec("some\nmultiline\ntext", "1")),
            format!(
                concat!(
                    "a=1\n",
                    "  > msg={vh}\n",
                    "    {vi}some\n",
                    "    {vi}multiline\n",
                    "    {vi}text",
                ),
                vh = EXPANDED_VALUE_HEADER,
                vi = EXPANDED_VALUE_INDENT,
            )
        );

        formatter.cfg.theme = settings().theme;

        assert_eq!(
            formatter.format_to_string(&rec("some\nmultiline\ntext", "1")),
            format!(
                concat!(
                    "\u{1b}[0;32ma\u{1b}[0;2m=\u{1b}[0;94m1\u{1b}[0;32m\u{1b}[0m\n",
                    "  \u{1b}[0;2m> \u{1b}[0;32mmsg\u{1b}[0;2m=\u{1b}[0;39m\u{1b}[0;2m{vh}\u{1b}[0m\n",
                    "  \u{1b}[0;2m  {vi}\u{1b}[0msome\n",
                    "  \u{1b}[0;2m  {vi}\u{1b}[0mmultiline\n",
                    "  \u{1b}[0;2m  {vi}\u{1b}[0mtext\u{1b}[0m",
                ),
                vh = EXPANDED_VALUE_HEADER,
                vi = EXPANDED_VALUE_INDENT,
            )
        );
    }

    #[test]
    fn test_expand_without_message() {
        let rec = |f, ts| Record {
            ts,
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[("a", EncodedString::raw(f).into())]).unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        let ts = Timestamp::new("2000-01-02T03:04:05.123Z");

        let formatter = RecordFormatter::new(settings().with(|s| {
            s.theme = Default::default();
            s.expansion.mode = ExpandOption::Always;
        }));

        assert_eq!(
            formatter.format_to_string(&rec("1", None)),
            format!("{mh}\n  > a=1", mh = EXPANDED_MESSAGE_HEADER)
        );
        assert_eq!(
            formatter.format_to_string(&rec("1", Some(ts))),
            format!(
                concat!("2000-01-02 03:04:05.123 {mh}\n", "                          > a=1"),
                mh = EXPANDED_MESSAGE_HEADER
            )
        );
    }

    #[test]
    fn test_format_uuid() {
        let rec = |value| Record {
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[("a", EncodedString::raw(value).into())]).unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            format_no_color(&rec("243e020d-11d6-42f6-b4cd-b4586057b9a2")),
            "a=243e020d-11d6-42f6-b4cd-b4586057b9a2"
        );
    }

    #[test]
    fn test_format_int_string() {
        let rec = |value| Record {
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[("a", EncodedString::json(value).into())]).unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(format_no_color(&rec(r#""243""#)), r#"a="243""#);
    }

    #[test]
    fn test_format_unparsable_time() {
        let rec = |ts, msg| Record {
            ts: Some(Timestamp::new(ts)),
            level: Some(Level::Info),
            message: Some(EncodedString::raw(msg).into()),
            ..Default::default()
        };

        assert_eq!(
            format_no_color(&rec("some-unparsable-time", "some-msg")),
            "|INF| some-msg ts=some-unparsable-time"
        );
    }

    #[test]
    fn test_format_value_with_eq() {
        let rec = |value| Record {
            fields: RecordFields {
                head: heapless::Vec::from_slice(&[("a", EncodedString::raw(value).into())]).unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(format_no_color(&rec("x=y")), r#"a="x=y""#);
        assert_eq!(format_no_color(&rec("|=>")), r#"a="|=>""#);
    }

    #[test]
    fn test_value_format_auto() {
        let vf = string::ValueFormatAuto::new(EncodedString::raw("test"));
        let mut buf = Vec::new();
        let result = vf.format(&mut buf).unwrap();
        assert_eq!(buf, b"test");
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_message_format_auto_empty() {
        let vf = string::MessageFormatAuto::new(EncodedString::raw(""));
        let mut buf = Vec::new();
        let result = vf.format(&mut buf).unwrap();
        assert_eq!(buf, br#""""#);
        assert_eq!(result.is_ok(), true);
    }
}
