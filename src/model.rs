// std imports
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
    iter::IntoIterator,
    marker::PhantomData,
    ops::Range,
    str::FromStr,
    sync::Arc,
};

// third-party imports
use chrono::{DateTime, Utc};
use enumset::{EnumSet, EnumSetType, enum_set};
use regex::Regex;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{self as json};
use titlecase::titlecase;
use wildflower::Pattern;

// other local crates
use encstr::{AnyEncodedString, EncodedString};
use serde_logfmt::logfmt;

// local imports
use crate::{
    app::{InputFormat, UnixTimestampUnit},
    error::{Error, Result},
    level::{self},
    serdex::StreamDeserializerWithOffsets,
    settings::PredefinedFields,
    timestamp::Timestamp,
    types::FieldKind,
};

// test imports
#[cfg(test)]
use crate::testing::Sample;

// ---

pub use level::Level;

pub const MAX_NUMBER_LEN: usize = 39;

// ---

#[inline]
pub fn looks_like_number(value: &[u8]) -> bool {
    if value.len() > MAX_NUMBER_LEN {
        return false;
    }

    crate::number::looks_like_number(value)
}

// ---

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum RawValue<'a> {
    String(EncodedString<'a>),
    Null,
    Boolean(bool),
    Number(&'a str),
    Object(RawObject<'a>),
    Array(RawArray<'a>),
}

impl<'a> RawValue<'a> {
    #[inline]
    pub fn auto(value: &'a str) -> Self {
        match value.as_bytes() {
            [b'"', ..] => Self::String(EncodedString::Json(value.into())),
            b"false" => Self::Boolean(false),
            b"true" => Self::Boolean(true),
            b"null" => Self::Null,
            _ if looks_like_number(value.as_bytes()) => Self::Number(value),
            _ => Self::String(EncodedString::Raw(value.into())),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::String(value) => value.is_empty(),
            Self::Null => true,
            Self::Boolean(_) => false,
            Self::Number(_) => false,
            Self::Object(value) => value.is_empty(),
            Self::Array(value) => value.is_empty(),
        }
    }

    #[inline]
    pub fn raw_str(&self) -> &'a str {
        match self {
            Self::String(value) => value.source(),
            Self::Null => "null",
            Self::Boolean(true) => "true",
            Self::Boolean(false) => "false",
            Self::Number(value) => value,
            Self::Object(value) => value.get(),
            Self::Array(value) => value.get(),
        }
    }

    #[inline]
    pub fn format_readable(&self, buf: &mut Vec<u8>) {
        match self {
            Self::String(value) => value.decode(buf).unwrap(),
            Self::Null => buf.extend(b"null"),
            Self::Boolean(true) => buf.extend(b"true"),
            Self::Boolean(false) => buf.extend(b"false"),
            Self::Number(value) => buf.extend(value.as_bytes()),
            Self::Object(_) => buf.extend(b"{?}"),
            Self::Array(_) => buf.extend(b"[?]"),
        }
    }

    #[inline]
    pub fn parse<T: Deserialize<'a>>(&self) -> Result<T> {
        let (s, is_json) = match self {
            Self::String(EncodedString::Json(value)) => (value.source(), true),
            Self::String(EncodedString::Raw(value)) => (value.source(), false),
            Self::Object(value) => (value.get(), true),
            Self::Array(value) => (value.get(), true),
            Self::Null => ("null", true),
            Self::Boolean(true) => ("true", false),
            Self::Boolean(false) => ("false", false),
            Self::Number(value) => (*value, false),
        };

        if is_json {
            json::from_str(s).map_err(Error::JsonParseError)
        } else {
            logfmt::from_str(s).map_err(Error::LogfmtParseError)
        }
    }

    #[inline]
    pub fn is_byte_code(&self) -> bool {
        let s = self.raw_str();
        let v = s.as_bytes();
        match v.len() {
            1 => v[0].is_ascii_digit(),
            2 => v[0].is_ascii_digit() && v[1].is_ascii_digit(),
            3 => &b"100"[..] <= v && v <= &b"255"[..],
            _ => false,
        }
    }

    #[inline]
    pub fn parse_byte_code(&self) -> u8 {
        let s = self.raw_str();
        match s.as_bytes() {
            [a] => a - b'0',
            [a, b] => (a - b'0') * 10 + (b - b'0'),
            [a, b, c] => (a - b'0') * 100 + (b - b'0') * 10 + (c - b'0'),
            _ => 0,
        }
    }

    #[inline]
    pub fn rough_complexity(&self) -> usize {
        match self {
            Self::String(EncodedString::Json(value)) => 4 + value.source().len(),
            Self::String(EncodedString::Raw(value)) => value.source().len(),
            Self::Null => 4,
            Self::Boolean(false) => 5,
            Self::Boolean(true) => 4,
            Self::Number(value) => value.len(),
            Self::Object(value) => value.rough_complexity(),
            Self::Array(value) => value.rough_complexity(),
        }
    }
}

impl<'a> From<EncodedString<'a>> for RawValue<'a> {
    #[inline]
    fn from(value: EncodedString<'a>) -> Self {
        Self::String(value)
    }
}

impl<'a> From<&'a json::value::RawValue> for RawValue<'a> {
    #[inline]
    fn from(value: &'a json::value::RawValue) -> Self {
        match value.get().as_bytes() {
            [b'"', ..] => Self::from(EncodedString::Json(value.get().into())),
            [b'0'..=b'9' | b'-' | b'+' | b'.', ..] => Self::Number(value.get()),
            [b'{', ..] => Self::from(RawObject::Json(value)),
            [b'[', ..] => Self::from(RawArray::Json(value)),
            [b't', ..] => Self::Boolean(true),
            [b'f', ..] => Self::Boolean(false),
            [b'n', ..] => Self::Null,
            _ => Self::String(EncodedString::raw(value.get())),
        }
    }
}

impl<'a> From<&'a logfmt::raw::RawValue> for RawValue<'a> {
    #[inline]
    fn from(value: &'a logfmt::raw::RawValue) -> Self {
        if let [b'"', ..] = value.get().as_bytes() {
            Self::from(EncodedString::Json(value.get().into()))
        } else {
            Self::from(EncodedString::Raw(value.get().into()))
        }
    }
}

impl<'a> From<RawObject<'a>> for RawValue<'a> {
    #[inline]
    fn from(value: RawObject<'a>) -> Self {
        Self::Object(value)
    }
}

impl<'a> From<RawArray<'a>> for RawValue<'a> {
    #[inline]
    fn from(value: RawArray<'a>) -> Self {
        Self::Array(value)
    }
}

// ---

#[derive(Clone, Copy, Debug)]
pub enum RawObject<'a> {
    Json(&'a json::value::RawValue),
}

impl<'a> RawObject<'a> {
    #[inline]
    pub fn get(&self) -> &'a str {
        match self {
            Self::Json(value) => value.get(),
        }
    }

    #[inline]
    pub fn parse(&self) -> Result<Object<'a>> {
        match self {
            Self::Json(value) => Object::from_json(value.get()),
        }
    }

    #[inline]
    pub fn parse_into(&self, target: &mut Object<'a>) -> Result<()> {
        match self {
            Self::Json(value) => target.set_from_json(value.get()),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Json(value) => json_match(value, "{}"),
        }
    }

    #[inline]
    pub fn rough_complexity(&self) -> usize {
        match self {
            Self::Json(value) => 4 + value.get().len() * 3 / 2,
        }
    }
}

impl<'a> From<&'a json::value::RawValue> for RawObject<'a> {
    #[inline]
    fn from(value: &'a json::value::RawValue) -> Self {
        Self::Json(value)
    }
}

impl PartialEq for RawObject<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for RawObject<'_> {}

// ---

#[derive(Clone, Copy, Debug)]
pub enum RawArray<'a> {
    Json(&'a json::value::RawValue),
}

impl<'a> RawArray<'a> {
    #[inline]
    pub fn get(&self) -> &'a str {
        match self {
            Self::Json(value) => value.get(),
        }
    }

    #[inline]
    pub fn parse<const N: usize>(&self) -> Result<Array<'a, N>> {
        Array::from_json(self.get())
    }

    #[inline]
    pub fn parse_into<const N: usize>(&self, target: &mut Array<'a, N>) -> Result<()> {
        target.set_from_json(self.get())
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Json(value) => json_match(value, "[]"),
        }
    }

    #[inline]
    pub fn rough_complexity(&self) -> usize {
        match self {
            Self::Json(value) => 4 + value.get().len() * 5 / 4,
        }
    }
}

impl<'a> From<&'a json::value::RawValue> for RawArray<'a> {
    #[inline]
    fn from(value: &'a json::value::RawValue) -> Self {
        Self::Json(value)
    }
}

impl PartialEq for RawArray<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for RawArray<'_> {}

// ---

#[derive(Default)]
pub struct Record<'a> {
    pub ts: Option<Timestamp<'a>>,
    pub message: Option<RawValue<'a>>,
    pub level: Option<Level>,
    pub logger: Option<&'a str>,
    pub caller: Caller<'a>,
    pub(crate) fields: RecordFields<'a>,
    pub(crate) predefined: heapless::Vec<(&'a str, RawValue<'a>), MAX_PREDEFINED_FIELDS>,
}

impl<'a> Record<'a> {
    #[inline]
    pub fn fields(&self) -> impl Iterator<Item = &(&'a str, RawValue<'a>)> {
        self.fields.iter()
    }

    #[inline]
    pub fn fields_for_search(&self) -> impl Iterator<Item = &(&'a str, RawValue<'a>)> {
        self.fields().chain(self.predefined.iter())
    }

    #[inline]
    pub fn matches<F: RecordFilter>(&self, filter: F) -> bool {
        filter.apply(self)
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            ts: None,
            message: None,
            level: None,
            logger: None,
            caller: Default::default(),
            fields: RecordFields::with_capacity(capacity),
            predefined: heapless::Vec::new(),
        }
    }
}

#[cfg(test)]
impl Sample for Record<'static> {
    fn sample() -> Self {
        Self {
            message: Some(RawValue::String(EncodedString::raw("test message"))),
            caller: Caller::with_name("test-caller"),
            ..Default::default()
        }
    }
}

pub type RecordFields<'a> = heapopt::Vec<(&'a str, RawValue<'a>), RECORD_EXTRA_CAPACITY>;

// ---

pub trait RecordWithSourceConstructor<'r, 's> {
    fn with_source(&'r self, source: &'s [u8]) -> RecordWithSource<'r, 's>;
}

// ---

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Caller<'a> {
    pub name: &'a str,
    pub file: &'a str,
    pub line: &'a str,
}

impl<'a> Caller<'a> {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_name(name: &'a str) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub fn with_file_line(file: &'a str, line: &'a str) -> Self {
        Self {
            file,
            line,
            ..Default::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty() && self.file.is_empty() && self.line.is_empty()
    }
}

// ---

pub struct RecordWithSource<'r, 's> {
    pub record: &'r Record<'s>,
    pub source: &'s [u8],
}

impl<'r, 's> RecordWithSource<'r, 's> {
    #[inline]
    pub fn new(record: &'r Record<'s>, source: &'s [u8]) -> Self {
        Self { record, source }
    }
}

impl<'r, 's> RecordWithSourceConstructor<'r, 's> for Record<'s> {
    #[inline]
    fn with_source(&'r self, source: &'s [u8]) -> RecordWithSource<'r, 's> {
        RecordWithSource::new(self, source)
    }
}

// ---

pub trait RecordFilter {
    fn apply<'a>(&self, record: &Record<'a>) -> bool;

    #[inline]
    fn and<F>(self, rhs: F) -> RecordFilterAnd<Self, F>
    where
        Self: Sized,
        F: RecordFilter,
    {
        RecordFilterAnd { lhs: self, rhs }
    }

    #[inline]
    fn or<F>(self, rhs: F) -> RecordFilterOr<Self, F>
    where
        Self: Sized,
        F: RecordFilter,
    {
        RecordFilterOr { lhs: self, rhs }
    }
}

impl<T: RecordFilter + ?Sized> RecordFilter for Box<T> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl<T: RecordFilter + ?Sized> RecordFilter for Arc<T> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl<T: RecordFilter> RecordFilter for &T {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl RecordFilter for Level {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        record.level.is_some_and(|x| x <= *self)
    }
}

impl<T: RecordFilter> RecordFilter for Option<T> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        if let Some(filter) = self {
            filter.apply(record)
        } else {
            true
        }
    }
}

// ---

pub struct RecordFilterAnd<L: RecordFilter, R: RecordFilter> {
    lhs: L,
    rhs: R,
}

impl<L: RecordFilter, R: RecordFilter> RecordFilter for RecordFilterAnd<L, R> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        self.lhs.apply(record) && self.rhs.apply(record)
    }
}

// ---

pub struct RecordFilterOr<L: RecordFilter, R: RecordFilter> {
    lhs: L,
    rhs: R,
}

impl<L: RecordFilter, R: RecordFilter> RecordFilter for RecordFilterOr<L, R> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        self.lhs.apply(record) || self.rhs.apply(record)
    }
}

// ---

pub struct RecordFilterNone;

impl RecordFilter for RecordFilterNone {
    #[inline]
    fn apply<'a>(&self, _: &Record<'a>) -> bool {
        true
    }
}

// ---

pub struct ParserSettings {
    unix_ts_unit: Option<UnixTimestampUnit>,
    level: Vec<(HashMap<String, Level>, Option<Level>)>,
    blocks: Vec<ParserSettingsBlock>,
    ignore: Vec<Pattern<String>>,
}

impl ParserSettings {
    pub fn new<'a, I: IntoIterator<Item = &'a String>>(
        predefined: &PredefinedFields,
        ignore: I,
        unix_ts_unit: Option<UnixTimestampUnit>,
    ) -> Self {
        let mut result = Self {
            unix_ts_unit,
            level: Vec::new(),
            blocks: vec![ParserSettingsBlock::default()],
            ignore: ignore.into_iter().map(|x| Pattern::new(x.to_string())).collect(),
        };

        result.init(predefined);
        result
    }

    fn init(&mut self, pf: &PredefinedFields) {
        self.build_block(0, &pf.time.names, FieldSettings::Time, 0);
        self.build_block(0, &pf.message.names, FieldSettings::Message, 0);
        self.build_block(0, &pf.logger.names, FieldSettings::Logger, 0);
        self.build_block(0, &pf.caller.names, FieldSettings::Caller, 0);
        self.build_block(0, &pf.caller_file.names, FieldSettings::CallerFile, 0);
        self.build_block(0, &pf.caller_line.names, FieldSettings::CallerLine, 0);

        let mut j = 0;
        for variant in &pf.level.variants {
            let Some(variant) = variant.resolve() else {
                continue;
            };

            let mut mapping = HashMap::new();
            for (level, values) in &variant.values {
                for value in values {
                    mapping.insert(value.clone(), *level);
                    mapping.insert(value.to_lowercase(), *level);
                    mapping.insert(value.to_uppercase(), *level);
                    mapping.insert(titlecase(value), *level);
                }
            }
            let k = self.level.len();
            self.level.push((mapping.clone(), variant.level));
            self.build_block(0, &variant.names, FieldSettings::Level(k), j);
            j += variant.names.len();
        }
    }

    fn build_block<'a, N: IntoIterator<Item = &'a String>>(
        &mut self,
        n: usize,
        names: N,
        settings: FieldSettings,
        priority: usize,
    ) {
        for (i, name) in names.into_iter().enumerate() {
            self.build_block_for_name(n, name, settings, priority + i)
        }
    }

    fn build_block_for_name(&mut self, n: usize, name: &str, settings: FieldSettings, priority: usize) {
        self.blocks[n].fields.insert(name.to_owned(), (settings, priority));
        let mut remainder = name;
        while let Some(k) = remainder.rfind('.') {
            let (name, nested) = name.split_at(k);
            let nested = &nested[1..];

            let nest = self.blocks[n]
                .fields
                .get(name)
                .and_then(|f| {
                    if let FieldSettings::Nested(nest) = f.0 {
                        Some(nest)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    let nest = self.blocks.len();
                    self.blocks.push(ParserSettingsBlock::default());
                    self.blocks[n]
                        .fields
                        .insert(name.to_string(), (FieldSettings::Nested(nest), priority));
                    nest
                });

            self.build_block_for_name(nest, nested, settings, priority);
            remainder = name;
        }
    }

    #[inline]
    fn apply<'a>(&self, key: &'a str, value: RawValue<'a>, to: &mut Record<'a>, pc: &mut PriorityController) {
        self.blocks[0].apply(self, key, value, to, pc, true);
    }

    #[inline]
    fn apply_each<'a, 'i, I>(&self, items: I, to: &mut Record<'a>)
    where
        I: IntoIterator<Item = &'i (&'a str, RawValue<'a>)>,
        'a: 'i,
    {
        let mut pc = PriorityController::default();
        self.apply_each_ctx(items, to, &mut pc);
    }

    #[inline]
    fn apply_each_ctx<'a, 'i, I>(&self, items: I, to: &mut Record<'a>, pc: &mut PriorityController)
    where
        I: IntoIterator<Item = &'i (&'a str, RawValue<'a>)>,
        'a: 'i,
    {
        for (key, value) in items {
            self.apply(key, *value, to, pc)
        }
    }
}

impl Default for ParserSettings {
    #[inline]
    fn default() -> Self {
        Self::new(&PredefinedFields::default(), Vec::new(), None)
    }
}

// ---

#[derive(Default, Debug)]
struct ParserSettingsBlock {
    fields: HashMap<String, (FieldSettings, usize)>,
}

impl ParserSettingsBlock {
    fn apply<'a>(
        &self,
        ps: &ParserSettings,
        key: &'a str,
        value: RawValue<'a>,
        to: &mut Record<'a>,
        pc: &mut PriorityController,
        is_root: bool,
    ) -> bool {
        let done = match self.fields.get(key) {
            Some((field, priority)) => {
                let kind = field.kind();
                if let Some(kind) = kind {
                    pc.prioritize(kind, *priority, |pc| field.apply_ctx(ps, value, to, pc))
                } else {
                    field.apply_ctx(ps, value, to, pc)
                }
            }
            None => false,
        };
        if is_root && done {
            to.predefined.push((key, value)).ok();
        }
        if done || !is_root {
            return done;
        }

        for pattern in &ps.ignore {
            if pattern.matches(key) {
                return false;
            }
        }
        to.fields.push((key, value));
        false
    }

    #[inline]
    fn apply_each_ctx<'a, 'i, I>(
        &self,
        ps: &ParserSettings,
        items: I,
        to: &mut Record<'a>,
        ctx: &mut PriorityController,
        is_root: bool,
    ) -> bool
    where
        I: IntoIterator<Item = &'i (&'a str, RawValue<'a>)>,
        'a: 'i,
    {
        let mut any_matched = false;
        for (key, value) in items {
            any_matched |= self.apply(ps, key, *value, to, ctx, is_root);
        }
        any_matched
    }
}

// ---

#[derive(Default)]
struct PriorityController {
    time: Option<usize>,
    level: Option<usize>,
    logger: Option<usize>,
    message: Option<usize>,
    caller: Option<usize>,
    caller_file: Option<usize>,
    caller_line: Option<usize>,
}

impl PriorityController {
    #[inline]
    fn prioritize<F: FnOnce(&mut Self) -> bool>(&mut self, kind: FieldKind, priority: usize, update: F) -> bool {
        let p = match kind {
            FieldKind::Time => &mut self.time,
            FieldKind::Level => &mut self.level,
            FieldKind::Logger => &mut self.logger,
            FieldKind::Message => &mut self.message,
            FieldKind::Caller => &mut self.caller,
            FieldKind::CallerFile => &mut self.caller_file,
            FieldKind::CallerLine => &mut self.caller_line,
        };

        if p.is_none() || Some(priority) <= *p {
            *p = Some(priority);
            update(self)
        } else {
            false
        }
    }
}

// ---

#[derive(Clone, Copy, Debug)]
enum FieldSettings {
    Time,
    Level(usize),
    Logger,
    Message,
    Caller,
    CallerFile,
    CallerLine,
    Nested(usize),
}

impl FieldSettings {
    fn apply<'a>(&self, ps: &ParserSettings, value: RawValue<'a>, to: &mut Record<'a>) -> bool {
        match *self {
            Self::Time => {
                let s = value.raw_str();
                let s = if !s.is_empty() && s.as_bytes()[0] == b'"' {
                    &s[1..s.len() - 1]
                } else {
                    s
                };
                if !s.is_empty() {
                    let ts = Timestamp::new(s).with_unix_unit(ps.unix_ts_unit);
                    to.ts = Some(ts);
                    true
                } else {
                    false
                }
            }
            Self::Level(i) => {
                let value = value.parse().ok().unwrap_or_else(|| value.raw_str());
                if let Some(level) = ps.level[i].0.get(value) {
                    to.level = Some(*level);
                    true
                } else {
                    to.level = ps.level[i].1;
                    false
                }
            }
            Self::Logger => {
                to.logger = value.parse::<&str>().ok().filter(|s| !s.is_empty());
                true
            }
            Self::Message => {
                to.message = Some(value);
                true
            }
            Self::Caller => {
                to.caller.name = value.parse::<&str>().ok().unwrap_or_default();
                true
            }
            Self::CallerFile => {
                to.caller.file = value.parse::<&str>().ok().unwrap_or_default();
                true
            }
            Self::CallerLine => {
                let value = match value {
                    RawValue::Number(value) => value,
                    RawValue::String(_) => {
                        if let Some(value) = value.parse::<&str>().ok().filter(|x| !x.is_empty()) {
                            value
                        } else {
                            return false;
                        }
                    }
                    _ => return false,
                };

                to.caller.line = value;
                true
            }
            Self::Nested(_) => false,
        }
    }

    #[inline]
    fn apply_ctx<'a>(
        &self,
        ps: &ParserSettings,
        value: RawValue<'a>,
        to: &mut Record<'a>,
        ctx: &mut PriorityController,
    ) -> bool {
        match *self {
            Self::Nested(nested) => match value {
                RawValue::Object(value) => {
                    let mut object = Object::default();
                    if value.parse_into(&mut object).is_ok() {
                        ps.blocks[nested].apply_each_ctx(ps, object.fields.iter(), to, ctx, false);
                    }
                    false
                }
                _ => false,
            },
            _ => self.apply(ps, value, to),
        }
    }

    #[inline]
    fn kind(&self) -> Option<FieldKind> {
        match self {
            Self::Time => Some(FieldKind::Time),
            Self::Level(_) => Some(FieldKind::Level),
            Self::Logger => Some(FieldKind::Logger),
            Self::Message => Some(FieldKind::Message),
            Self::Caller => Some(FieldKind::Caller),
            Self::CallerFile => Some(FieldKind::CallerFile),
            Self::CallerLine => Some(FieldKind::CallerLine),
            Self::Nested(_) => None,
        }
    }
}

// ---

pub struct Parser {
    settings: ParserSettings,
}

impl Parser {
    #[inline]
    pub fn new(settings: ParserSettings) -> Self {
        Self { settings }
    }

    #[inline]
    pub fn parse<'a>(&self, record: &RawRecord<'a>) -> Record<'a> {
        let fields = record.fields();
        let count = fields.size_hint().1.unwrap_or(0);
        let mut record = Record::<'a>::with_capacity(count);

        self.settings.apply_each(fields, &mut record);

        record
    }
}

// ---

#[derive(Default)]
pub struct RawRecord<'a> {
    fields: RawRecordFields<'a>,
}

impl<'a> RawRecord<'a> {
    #[inline]
    pub fn fields(&self) -> impl Iterator<Item = &(&'a str, RawValue<'a>)> {
        self.fields.iter()
    }

    #[inline]
    pub fn parser() -> RawRecordParser {
        RawRecordParser::new()
    }
}

impl<'a> Deserialize<'a> for RawRecord<'a> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        let mut target = Self::default();
        Self::deserialize_in_place(deserializer, &mut target)?;
        Ok(target)
    }

    #[inline]
    fn deserialize_in_place<D>(deserializer: D, target: &mut Self) -> std::result::Result<(), D::Error>
    where
        D: Deserializer<'a>,
    {
        const N: usize = RAW_RECORD_FIELDS_CAPACITY;
        deserializer.deserialize_map(ObjectVisitor::<json::value::RawValue, N>::new(&mut target.fields))
    }
}

// ---

pub type RawRecordFields<'a> = ObjectFields<'a, RAW_RECORD_FIELDS_CAPACITY>;

type ObjectFields<'a, const N: usize> = heapopt::Vec<(&'a str, RawValue<'a>), N>;

// ---

pub struct RawRecordParser {
    allow_prefix: bool,
    format: Option<InputFormat>,
}

impl Default for RawRecordParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RawRecordParser {
    #[inline]
    pub fn new() -> Self {
        Self {
            allow_prefix: false,
            format: None,
        }
    }

    #[inline]
    pub fn allow_prefix(self, value: bool) -> Self {
        Self {
            allow_prefix: value,
            ..self
        }
    }

    #[inline]
    pub fn format(self, format: Option<InputFormat>) -> Self {
        Self { format, ..self }
    }

    #[inline]
    pub fn parse<'a>(
        &self,
        line: &'a [u8],
    ) -> RawRecordStream<impl RawRecordIterator<'a> + use<'a>, impl RawRecordIterator<'a> + use<'a>> {
        let prefix = if self.allow_prefix && line.last() == Some(&b'}') {
            line.split(|c| *c == b'{').next().unwrap()
        } else {
            b""
        };

        let xn = prefix.len();
        let data = &line[xn..];

        let format = self.format.or_else(|| {
            if data.is_empty() {
                None
            } else if data[0] == b'{' {
                Some(InputFormat::Json)
            } else {
                Some(InputFormat::Logfmt)
            }
        });

        match format {
            None => RawRecordStream::Empty,
            Some(InputFormat::Json) => RawRecordStream::Json(RawRecordJsonStream {
                prefix,
                delegate: StreamDeserializerWithOffsets(json::Deserializer::from_slice(data).into_iter::<RawRecord>()),
            }),
            Some(InputFormat::Logfmt) => RawRecordStream::Logfmt(RawRecordLogfmtStream {
                line,
                prefix,
                done: false,
            }),
        }
    }
}

// ---

#[derive(Debug)]
pub enum RawRecordStream<Json, Logfmt> {
    Empty,
    Json(Json),
    Logfmt(Logfmt),
}

impl<'a, Json, Logfmt> RawRecordStream<Json, Logfmt>
where
    Json: RawRecordIterator<'a>,
    Logfmt: RawRecordIterator<'a>,
{
    #[inline]
    pub fn next(&mut self) -> Option<Result<AnnotatedRawRecord<'a>>> {
        match self {
            Self::Empty => None,
            Self::Json(stream) => stream.next(),
            Self::Logfmt(stream) => stream.next(),
        }
    }

    #[inline]
    pub fn collect_vec(&mut self) -> Vec<Result<AnnotatedRawRecord<'a>>> {
        let mut result = Vec::new();
        while let Some(item) = self.next() {
            result.push(item);
        }
        result
    }
}

// ---

pub trait RawRecordIterator<'a> {
    fn next(&mut self) -> Option<Result<AnnotatedRawRecord<'a>>>;
}

// ---

pub struct AnnotatedRawRecord<'a> {
    pub prefix: &'a [u8],
    pub record: RawRecord<'a>,
    pub offsets: Range<usize>,
}

// ---

struct RawRecordJsonStream<'a, R> {
    prefix: &'a [u8],
    delegate: StreamDeserializerWithOffsets<'a, R, RawRecord<'a>>,
}

impl<'a, R> RawRecordIterator<'a> for RawRecordJsonStream<'a, R>
where
    R: serde_json::de::Read<'a>,
{
    #[inline]
    fn next(&mut self) -> Option<Result<AnnotatedRawRecord<'a>>> {
        let pl = self.prefix.len();
        self.delegate.next().map(|res| {
            res.map(|(record, range)| {
                let range = range.start + pl..range.end + pl;
                AnnotatedRawRecord {
                    prefix: self.prefix,
                    record,
                    offsets: range,
                }
            })
            .map_err(Error::JsonParseError)
        })
    }
}

// ---

struct RawRecordLogfmtStream<'a> {
    line: &'a [u8],
    prefix: &'a [u8],
    done: bool,
}

impl<'a> RawRecordIterator<'a> for RawRecordLogfmtStream<'a> {
    #[inline]
    fn next(&mut self) -> Option<Result<AnnotatedRawRecord<'a>>> {
        if self.done {
            return None;
        }

        self.done = true;
        match logfmt::from_slice::<LogfmtRawRecord>(self.line) {
            Ok(record) => Some(Ok(AnnotatedRawRecord {
                prefix: self.prefix,
                record: record.0,
                offsets: 0..self.line.len(),
            })),
            Err(err) => Some(Err(err.into())),
        }
    }
}

// ---

struct ObjectVisitor<'a, 't, RV, const N: usize>
where
    RV: ?Sized + 'a,
{
    target: &'t mut ObjectFields<'a, N>,
    marker: PhantomData<fn(RV) -> RV>,
}

impl<'a, 't, RV, const N: usize> ObjectVisitor<'a, 't, RV, N>
where
    RV: ?Sized + 'a,
{
    #[inline]
    fn new(target: &'t mut ObjectFields<'a, N>) -> Self {
        Self {
            target,
            marker: PhantomData,
        }
    }
}

impl<'a, 'r, RV, const N: usize> Visitor<'a> for ObjectVisitor<'a, 'r, RV, N>
where
    RV: ?Sized + 'a,
    &'a RV: Deserialize<'a> + 'a,
    RawValue<'a>: From<&'a RV>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("json object")
    }

    #[inline]
    fn visit_map<M: MapAccess<'a>>(self, mut access: M) -> std::result::Result<Self::Value, M::Error> {
        self.target.clear();
        self.target.reserve(access.size_hint().unwrap_or(0));

        while let Some((key, value)) = access.next_entry::<&'a str, &RV>()? {
            self.target.push((key, value.into()));
        }

        Ok(())
    }
}

// ---

#[derive(Default)]
pub struct LogfmtRawRecord<'a>(pub RawRecord<'a>);

impl<'a> Deserialize<'a> for LogfmtRawRecord<'a> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        let mut target = Self::default();
        Self::deserialize_in_place(deserializer, &mut target)?;
        Ok(target)
    }

    #[inline]
    fn deserialize_in_place<D>(deserializer: D, target: &mut Self) -> std::result::Result<(), D::Error>
    where
        D: Deserializer<'a>,
    {
        const N: usize = RAW_RECORD_FIELDS_CAPACITY;
        deserializer.deserialize_map(ObjectVisitor::<logfmt::raw::RawValue, N>::new(&mut target.0.fields))
    }
}

// ---

#[derive(Debug)]
pub enum KeyMatch<'a> {
    Full,
    Partial(KeyMatcher<'a>),
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct KeyMatcher<'a> {
    key: &'a str,
}

impl<'a> KeyMatcher<'a> {
    #[inline]
    pub fn new(key: &'a str) -> Self {
        Self { key }
    }

    pub fn match_key<'b>(&'b self, key: &str) -> Option<KeyMatch<'a>> {
        let bytes = self.key.as_bytes();
        if bytes
            .iter()
            .zip(key.as_bytes().iter())
            .any(|(&x, &y)| Self::norm(x.into()) != Self::norm(y.into()))
        {
            return None;
        }

        if self.key.len() == key.len() {
            Some(KeyMatch::Full)
        } else if self.key.len() > key.len() {
            if bytes[key.len()] == b'.' {
                Some(KeyMatch::Partial(KeyMatcher::new(&self.key[key.len() + 1..])))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn index_matcher<'b>(&'b self) -> Option<(IndexMatcher, Option<KeyMatcher<'a>>)> {
        let bytes = self.key.as_bytes();
        match bytes {
            b"[]" => return Some((IndexMatcher::Any, None)),
            [b'[', b']', b'.', ..] => return Some((IndexMatcher::Any, Some(KeyMatcher::new(&self.key[3..])))),
            [b'[', ..] => {
                let tail = &bytes[1..];
                if let Some(pos) = tail.iter().position(|c| !c.is_ascii_digit()) {
                    if pos != 0 && tail[pos] == b']' && (pos >= tail.len() - 1 || tail[pos + 1] == b'.') {
                        if let Ok(idx) = unsafe { std::str::from_utf8_unchecked(&tail[..pos]) }.parse() {
                            let idx = IndexMatcher::Exact(idx);
                            if pos >= tail.len() - 1 {
                                return Some((idx, None));
                            } else {
                                return Some((
                                    idx,
                                    Some(KeyMatcher::new(unsafe {
                                        std::str::from_utf8_unchecked(&tail[pos + 2..])
                                    })),
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        };
        None
    }

    #[inline]
    fn norm(c: char) -> char {
        if c == '_' { '-' } else { c.to_ascii_lowercase() }
    }
}

// ---

pub enum IndexMatcher {
    Any,
    Exact(usize),
}

// ---

#[derive(Debug)]
pub enum Number {
    Integer(i128),
    Float(f64),
}

impl FromStr for Number {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self> {
        if s.contains('.') {
            Ok(Self::Float(s.parse().map_err(Error::from)?))
        } else {
            Ok(Self::Integer(s.parse().map_err(Error::from)?))
        }
    }
}

impl PartialEq<Number> for Number {
    #[inline]
    fn eq(&self, other: &Number) -> bool {
        match self {
            Self::Integer(a) => match other {
                Self::Integer(b) => a == b,
                Self::Float(b) => (*a as f64) == *b,
            },
            Self::Float(a) => match other {
                Self::Integer(b) => *a == (*b as f64),
                Self::Float(b) => a == b,
            },
        }
    }
}

impl Eq for Number {}

impl From<i128> for Number {
    #[inline]
    fn from(value: i128) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for Number {
    #[inline]
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl PartialOrd<Number> for Number {
    #[inline]
    fn partial_cmp(&self, other: &Number) -> Option<Ordering> {
        match self {
            Self::Integer(a) => match other {
                Self::Integer(b) => a.partial_cmp(b),
                Self::Float(b) => (*a as f64).partial_cmp(b),
            },
            Self::Float(a) => match other {
                Self::Integer(b) => a.partial_cmp(&(*b as f64)),
                Self::Float(b) => a.partial_cmp(b),
            },
        }
    }
}

// ---

#[derive(Debug)]
pub enum NumericOp {
    Eq(Number),
    Ne(Number),
    Gt(Number),
    Ge(Number),
    Lt(Number),
    Le(Number),
    In(Vec<Number>),
}

// ---

pub enum ValueMatchPolicy {
    Exact(String),
    SubString(String),
    RegularExpression(Regex),
    In(HashSet<String>),
    WildCard(Pattern<String>),
    Numerically(NumericOp),
    Any,
}

impl ValueMatchPolicy {
    fn matches(&self, subject: &str) -> bool {
        match self {
            Self::Exact(pattern) => subject == pattern,
            Self::SubString(pattern) => subject.contains(pattern),
            Self::RegularExpression(pattern) => pattern.is_match(subject),
            Self::In(patterns) => patterns.contains(subject),
            Self::WildCard(pattern) => pattern.matches(subject),
            Self::Numerically(op) => {
                if let Ok(value) = subject.parse::<Number>() {
                    match op {
                        NumericOp::Eq(pattern) => value == *pattern,
                        NumericOp::Ne(pattern) => value != *pattern,
                        NumericOp::Gt(pattern) => value > *pattern,
                        NumericOp::Ge(pattern) => value >= *pattern,
                        NumericOp::Lt(pattern) => value < *pattern,
                        NumericOp::Le(pattern) => value <= *pattern,
                        NumericOp::In(patterns) => patterns.contains(&value),
                    }
                } else {
                    false
                }
            }
            Self::Any => true,
        }
    }
}

// ---

#[derive(Debug)]
pub enum FieldFilterKey<S> {
    Predefined(FieldKind),
    Custom(S),
}

impl FieldFilterKey<String> {
    #[inline]
    pub fn borrowed(&self) -> FieldFilterKey<&str> {
        match self {
            FieldFilterKey::Predefined(kind) => FieldFilterKey::Predefined(*kind),
            FieldFilterKey::Custom(key) => FieldFilterKey::Custom(key.as_str()),
        }
    }
}

impl FieldFilterKey<&str> {
    #[inline]
    pub fn to_owned(&self) -> FieldFilterKey<String> {
        match self {
            FieldFilterKey::Predefined(kind) => FieldFilterKey::Predefined(*kind),
            FieldFilterKey::Custom(key) => FieldFilterKey::Custom((*key).to_owned()),
        }
    }

    pub fn parse(text: &str) -> Result<FieldFilterKey<&str>> {
        Ok(match text {
            "message" | "msg" => FieldFilterKey::Predefined(FieldKind::Message),
            "logger" => FieldFilterKey::Predefined(FieldKind::Logger),
            "caller" => FieldFilterKey::Predefined(FieldKind::Caller),
            _ => FieldFilterKey::Custom(text.trim_start_matches('.')),
        })
    }
}

#[derive(EnumSetType, Debug)]
pub(crate) enum FieldFilterFlag {
    Negate,
    IncludeAbsent,
}

pub(crate) type FieldFilterFlags = EnumSet<FieldFilterFlag>;

// ---

pub struct FieldFilter {
    key: FieldFilterKey<String>,
    match_policy: ValueMatchPolicy,
    flags: FieldFilterFlags,
    flat_key: bool,
}

impl FieldFilter {
    pub(crate) fn new(key: FieldFilterKey<&str>, match_policy: ValueMatchPolicy, flags: FieldFilterFlags) -> Self {
        Self {
            key: match key {
                FieldFilterKey::Predefined(kind) => FieldFilterKey::Predefined(kind),
                FieldFilterKey::Custom(key) => FieldFilterKey::Custom(key.chars().map(KeyMatcher::norm).collect()),
            },
            match_policy,
            flags,
            flat_key: match key {
                FieldFilterKey::Predefined(_) => true,
                FieldFilterKey::Custom(key) => !key.contains('.'),
            },
        }
    }

    pub(crate) fn parse(text: &str) -> Result<Self> {
        let parse = |key, value| {
            let (key, match_policy, flags) = Self::parse_mp_op(key, value)?;
            let key = FieldFilterKey::parse(key)?;
            Ok(Self::new(key, match_policy, flags))
        };

        if let Some(index) = text.find('=') {
            return parse(&text[0..index], &text[index + 1..]);
        }

        if let Some(index) = text.find(':') {
            return parse(&text[0..index], &text[index + 1..]);
        }

        Err(Error::WrongFieldFilter(text.into()))
    }

    fn parse_mp_op<'k>(key: &'k str, value: &str) -> Result<(&'k str, ValueMatchPolicy, FieldFilterFlags)> {
        let flags = |key: &'k str| {
            let (key, flags) = if let Some(key) = key.strip_suffix('!') {
                (key, FieldFilterFlag::Negate.into())
            } else {
                (key, FieldFilterFlags::empty())
            };
            if let Some(key) = key.strip_suffix('?') {
                (key, flags | enum_set!(FieldFilterFlag::IncludeAbsent))
            } else {
                (key, flags)
            }
        };
        Ok(if let Some(key) = key.strip_suffix('~') {
            if let Some(key) = key.strip_suffix('~') {
                let (key, flags) = flags(key);
                (key, ValueMatchPolicy::RegularExpression(value.parse()?), flags)
            } else {
                let (key, flags) = flags(key);
                (key, ValueMatchPolicy::SubString(value.into()), flags)
            }
        } else {
            let (key, flags) = flags(key);
            (key, ValueMatchPolicy::Exact(value.into()), flags)
        })
    }

    #[inline]
    fn match_custom_key<'a>(&'a self, key: &str) -> Option<KeyMatch<'a>> {
        if let FieldFilterKey::Custom(k) = &self.key {
            if self.flat_key && k.len() != key.len() {
                return None;
            }

            KeyMatcher::new(k).match_key(key)
        } else {
            None
        }
    }

    fn match_value(&self, value: &str, escaped: bool) -> bool {
        let apply = |value| {
            let result = self.match_policy.matches(value);
            if self.flags.contains(FieldFilterFlag::Negate) {
                !result
            } else {
                result
            }
        };
        if escaped {
            if let Ok(value) = json::from_str::<&str>(value) {
                return apply(value);
            } else if let Ok(value) = json::from_str::<String>(value) {
                return apply(&value);
            }
        }

        apply(value)
    }

    // Returns
    // * `None` if subkey does not match
    // * `Some(true)` if the subkey matches and the value matches
    // * `Some(false)` if the subkey matches but the value doesn't match
    fn match_value_partial<'a>(&self, subkey: KeyMatcher, value: RawValue<'a>) -> Option<bool> {
        match value {
            RawValue::Object(value) => {
                let mut item = Object::default();
                value.parse_into(&mut item).ok();
                for (k, v) in item.fields.iter() {
                    match subkey.match_key(k) {
                        None => {
                            continue;
                        }
                        Some(KeyMatch::Full) => {
                            let s = v.raw_str();
                            return Some(self.match_value(s, s.starts_with('"')));
                        }
                        Some(KeyMatch::Partial(subkey)) => {
                            return self.match_value_partial(subkey, *v);
                        }
                    }
                }
            }
            RawValue::Array(value) => {
                if let Some((index_matcher, tail)) = subkey.index_matcher() {
                    let matches = |item: RawValue<'a>| {
                        if let Some(tail) = &tail {
                            self.match_value_partial(*tail, item)
                        } else {
                            let s = item.raw_str();
                            Some(self.match_value(s, s.starts_with('"')))
                        }
                    };

                    if let Ok(value) = value.parse::<128>() {
                        match index_matcher {
                            IndexMatcher::Any => {
                                for item in value.iter() {
                                    if let Some(true) = matches(*item) {
                                        return Some(true);
                                    }
                                }
                            }
                            IndexMatcher::Exact(idx) => {
                                if let Some(item) = value.items.get(idx) {
                                    return matches(*item);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }
}

impl RecordFilter for FieldFilter {
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        match &self.key {
            FieldFilterKey::Predefined(kind) => match kind {
                FieldKind::Time => {
                    if let Some(ts) = &record.ts {
                        self.match_value(ts.raw(), false)
                    } else {
                        false
                    }
                }
                FieldKind::Message => {
                    if let Some(message) = record.message {
                        self.match_value(
                            message.raw_str(),
                            matches!(message, RawValue::String(EncodedString::Json(_))),
                        )
                    } else {
                        false
                    }
                }
                FieldKind::Logger => {
                    if let Some(logger) = record.logger {
                        self.match_value(logger, false)
                    } else {
                        false
                    }
                }
                FieldKind::Caller => {
                    if !record.caller.name.is_empty() {
                        self.match_value(record.caller.name, false)
                    } else {
                        false
                    }
                }
                _ => true,
            },
            FieldFilterKey::Custom(_) => {
                let mut key_matched = false;
                for (k, v) in record.fields_for_search() {
                    match self.match_custom_key(k) {
                        None => {}
                        Some(KeyMatch::Full) => {
                            key_matched = true;
                            let s = v.raw_str();
                            let escaped = s.starts_with('"');
                            if self.match_value(s, escaped) {
                                return true;
                            }
                        }
                        Some(KeyMatch::Partial(subkey)) => match self.match_value_partial(subkey, *v) {
                            Some(true) => return true,
                            Some(false) => key_matched = true,
                            None => {}
                        },
                    }
                }
                !key_matched && self.flags.contains(FieldFilterFlag::IncludeAbsent)
            }
        }
    }
}

// ---

#[derive(Default)]
pub struct FieldFilterSet(Vec<FieldFilter>);

impl FieldFilterSet {
    pub fn new<T: AsRef<str>, I: IntoIterator<Item = T>>(items: I) -> Result<Self> {
        let mut fields = Vec::new();
        for i in items {
            fields.push(FieldFilter::parse(i.as_ref())?);
        }
        Ok(FieldFilterSet(fields))
    }
}

impl RecordFilter for FieldFilterSet {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        self.0.iter().all(|field| field.apply(record))
    }
}

// ---

#[derive(Default)]
pub struct Filter {
    pub fields: FieldFilterSet,
    pub level: Option<Level>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

impl Filter {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.fields.0.is_empty() && self.level.is_none() && self.since.is_none() && self.until.is_none()
    }
}

impl RecordFilter for Filter {
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        if self.since.is_some() || self.until.is_some() {
            if let Some(ts) = record.ts.as_ref().and_then(|ts| ts.parse()) {
                if let Some(since) = self.since {
                    if ts < since {
                        return false;
                    }
                }
                if let Some(until) = self.until {
                    if ts > until {
                        return false;
                    }
                }
            }
        }

        if let Some(bound) = &self.level {
            if let Some(level) = record.level.as_ref() {
                if level > bound {
                    return false;
                }
            } else {
                return false;
            }
        }

        if !self.fields.apply(record) {
            return false;
        }

        true
    }
}

// ---

#[derive(Default)]
pub struct Object<'a, const N: usize = 32> {
    pub fields: heapopt::Vec<(&'a str, RawValue<'a>), N>,
}

impl<'a, const N: usize> Object<'a, N> {
    #[inline]
    pub fn from_json(s: &'a str) -> Result<Self> {
        let mut result = Self::default();
        result.set_from_json(s)?;
        Ok(result)
    }

    #[inline]
    pub fn set_from_json(&mut self, s: &'a str) -> Result<()> {
        let visitor = ObjectVisitor::<json::value::RawValue, N>::new(&mut self.fields);
        let mut deserializer = json::Deserializer::from_str(s);
        deserializer.deserialize_map(visitor).map_err(Error::JsonParseError)
    }
}

#[derive(Default)]
pub struct Array<'a, const N: usize = 32> {
    items: heapopt::Vec<RawValue<'a>, N>,
}

impl<'a, const N: usize> Array<'a, N> {
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &RawValue<'a>> {
        self.items.iter()
    }

    #[inline]
    pub fn from_json(s: &'a str) -> Result<Self> {
        let mut result = Self::default();
        result.set_from_json(s)?;
        Ok(result)
    }

    #[inline]
    pub fn set_from_json(&mut self, s: &'a str) -> Result<()> {
        let visitor = ArrayVisitor::<json::value::RawValue, N>::new(&mut self.items);
        let mut deserializer = json::Deserializer::from_str(s);
        deserializer.deserialize_seq(visitor).map_err(Error::JsonParseError)
    }
}

struct ArrayVisitor<'a, 't, RV, const N: usize>
where
    RV: ?Sized + 'a,
{
    target: &'t mut heapopt::Vec<RawValue<'a>, N>,
    marker: PhantomData<fn(RV) -> RV>,
}
impl<'a, 't, RV, const N: usize> ArrayVisitor<'a, 't, RV, N>
where
    RV: ?Sized + 'a,
{
    #[inline]
    fn new(target: &'t mut heapopt::Vec<RawValue<'a>, N>) -> Self {
        Self {
            target,
            marker: PhantomData,
        }
    }
}

impl<'a, 't, RV, const N: usize> Visitor<'a> for ArrayVisitor<'a, 't, RV, N>
where
    RV: ?Sized + 'a,
    &'a RV: Deserialize<'a> + 'a,
    RawValue<'a>: From<&'a RV>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("json array")
    }

    #[inline]
    fn visit_seq<A: SeqAccess<'a>>(self, mut access: A) -> std::result::Result<Self::Value, A::Error> {
        while let Some(item) = access.next_element::<&RV>()? {
            self.target.push(item.into());
        }

        Ok(())
    }
}

// ---

#[inline]
fn is_json_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

#[inline]
fn json_match(value: &json::value::RawValue, s: &str) -> bool {
    value
        .get()
        .as_bytes()
        .iter()
        .filter(|&b| !is_json_ws(*b))
        .eq(s.as_bytes().iter())
}

// ---

pub(crate) const RECORD_EXTRA_CAPACITY: usize = 32;
const MAX_PREDEFINED_FIELDS: usize = 8;
const RAW_RECORD_FIELDS_CAPACITY: usize = RECORD_EXTRA_CAPACITY + MAX_PREDEFINED_FIELDS;

// ---

#[cfg(test)]
mod tests;
