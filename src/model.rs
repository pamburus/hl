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

// ---

pub use level::Level;

pub const MAX_NUMBER_LEN: usize = 39;

// ---

pub fn looks_like_number(value: &[u8]) -> bool {
    if value.len() == 0 || value.len() > MAX_NUMBER_LEN {
        return false;
    }

    let mut s = value;
    let mut n_dots = 0;
    if s[0] == b'-' {
        s = &s[1..];
    }
    s.len() != 0
        && s.iter().all(|&x| {
            if x == b'.' {
                n_dots += 1;
                n_dots <= 1
            } else {
                x.is_ascii_digit()
            }
        })
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
    pub caller: Option<Caller<'a>>,
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
            caller: None,
            fields: RecordFields::with_capacity(capacity),
            predefined: heapless::Vec::new(),
        }
    }
}

pub type RecordFields<'a> = heapopt::Vec<(&'a str, RawValue<'a>), RECORD_EXTRA_CAPACITY>;

// ---

pub trait RecordWithSourceConstructor<'r, 's> {
    fn with_source(&'r self, source: &'s [u8]) -> RecordWithSource<'r, 's>;
}

// ---

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Caller<'a> {
    Text(&'a str),
    FileLine(&'a str, &'a str),
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
        record.level.map_or(false, |x| x <= *self)
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
                    mapping.insert(value.clone(), level.clone());
                    mapping.insert(value.to_lowercase(), level.clone());
                    mapping.insert(value.to_uppercase(), level.clone());
                    mapping.insert(titlecase(value), level.clone());
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

    fn build_block_for_name(&mut self, n: usize, name: &String, settings: FieldSettings, priority: usize) {
        self.blocks[n].fields.insert(name.clone(), (settings, priority));
        let mut remainder = &name[..];
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

            self.build_block_for_name(nest, &nested.into(), settings, priority);
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
    ) {
        let done = match self.fields.get(key) {
            Some((field, priority)) => {
                let kind = field.kind();
                if let Some(kind) = kind {
                    pc.prioritize(kind, *priority, |pc| field.apply_ctx(ps, value, to, pc))
                } else {
                    field.apply_ctx(ps, value, to, pc);
                    false
                }
            }
            None => false,
        };
        if is_root && done {
            to.predefined.push((key, value)).ok();
        }
        if done || !is_root {
            return;
        }

        for pattern in &ps.ignore {
            if pattern.matches(key) {
                return;
            }
        }
        to.fields.push((key, value));
    }

    #[inline]
    fn apply_each_ctx<'a, 'i, I>(
        &self,
        ps: &ParserSettings,
        items: I,
        to: &mut Record<'a>,
        ctx: &mut PriorityController,
        is_root: bool,
    ) where
        I: IntoIterator<Item = &'i (&'a str, RawValue<'a>)>,
        'a: 'i,
    {
        for (key, value) in items {
            self.apply(ps, key, *value, to, ctx, is_root)
        }
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
                let s = if s.len() > 0 && s.as_bytes()[0] == b'"' {
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
                to.caller = value
                    .parse::<&str>()
                    .ok()
                    .filter(|x| !x.is_empty())
                    .map(|x| Caller::Text(x));
                true
            }
            Self::CallerFile => match &mut to.caller {
                None => {
                    to.caller = value
                        .parse::<&str>()
                        .ok()
                        .filter(|x| !x.is_empty())
                        .map(|x| Caller::FileLine(x, ""));
                    to.caller.is_some()
                }
                Some(Caller::FileLine(file, _)) => {
                    if let Some(value) = value.parse::<&str>().ok().filter(|x| !x.is_empty()) {
                        *file = value;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
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

                match &mut to.caller {
                    None => to.caller = Some(Caller::FileLine("", value)),
                    Some(Caller::FileLine(_, line)) => *line = value,
                    Some(Caller::Text(_)) => return false,
                }
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
                        true
                    } else {
                        false
                    }
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
        Ok(deserializer.deserialize_map(ObjectVisitor::<json::value::RawValue, N>::new(&mut target.fields))?)
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
            if data.len() == 0 {
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

#[derive(Debug)]
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
            .position(|(&x, &y)| Self::norm(x.into()) != Self::norm(y.into()))
            .is_some()
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

    #[inline]
    fn norm(c: char) -> char {
        if c == '_' { '-' } else { c.to_ascii_lowercase() }
    }
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
            Ok(Self::Float(s.parse().map_err(|e| Error::from(e))?))
        } else {
            Ok(Self::Integer(s.parse().map_err(|e| Error::from(e))?))
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
                if let Some(value) = subject.parse::<Number>().ok() {
                    match op {
                        NumericOp::Eq(pattern) => value == *pattern,
                        NumericOp::Ne(pattern) => value != *pattern,
                        NumericOp::Gt(pattern) => value > *pattern,
                        NumericOp::Ge(pattern) => value >= *pattern,
                        NumericOp::Lt(pattern) => value < *pattern,
                        NumericOp::Le(pattern) => value <= *pattern,
                        NumericOp::In(patterns) => patterns.iter().any(|pattern| value == *pattern),
                    }
                } else {
                    false
                }
            }
        }
    }
}

// ---

#[derive(Copy, Clone, Debug)]
pub(crate) enum UnaryBoolOp {
    None,
    Negate,
}

impl UnaryBoolOp {
    #[inline]
    fn apply(self, value: bool) -> bool {
        match self {
            Self::None => value,
            Self::Negate => !value,
        }
    }
}

impl Default for UnaryBoolOp {
    #[inline]
    fn default() -> Self {
        Self::None
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

// ---

pub struct FieldFilter {
    key: FieldFilterKey<String>,
    match_policy: ValueMatchPolicy,
    op: UnaryBoolOp,
    flat_key: bool,
}

impl FieldFilter {
    pub(crate) fn new(key: FieldFilterKey<&str>, match_policy: ValueMatchPolicy, op: UnaryBoolOp) -> Self {
        Self {
            key: match key {
                FieldFilterKey::Predefined(kind) => FieldFilterKey::Predefined(kind),
                FieldFilterKey::Custom(key) => FieldFilterKey::Custom(key.chars().map(KeyMatcher::norm).collect()),
            },
            match_policy,
            op,
            flat_key: match key {
                FieldFilterKey::Predefined(_) => true,
                FieldFilterKey::Custom(key) => !key.contains('.'),
            },
        }
    }

    pub(crate) fn parse(text: &str) -> Result<Self> {
        let parse = |key, value| {
            let (key, match_policy, op) = Self::parse_mp_op(key, value)?;
            let key = match key {
                "message" | "msg" => FieldFilterKey::Predefined(FieldKind::Message),
                "caller" => FieldFilterKey::Predefined(FieldKind::Caller),
                "logger" => FieldFilterKey::Predefined(FieldKind::Logger),
                _ => FieldFilterKey::Custom(key.trim_start_matches('.')),
            };
            Ok(Self::new(key, match_policy, op))
        };

        if let Some(index) = text.find('=') {
            return parse(&text[0..index], &text[index + 1..]);
        }

        if let Some(index) = text.find(':') {
            return parse(&text[0..index], &text[index + 1..]);
        }

        Err(Error::WrongFieldFilter(text.into()))
    }

    fn parse_mp_op<'k>(key: &'k str, value: &str) -> Result<(&'k str, ValueMatchPolicy, UnaryBoolOp)> {
        let key_op = |key: &'k str| {
            if let Some(key) = key.strip_suffix('!') {
                (key, UnaryBoolOp::Negate)
            } else {
                (key, UnaryBoolOp::None)
            }
        };
        Ok(if let Some(key) = key.strip_suffix('~') {
            if let Some(key) = key.strip_suffix('~') {
                let (key, op) = key_op(key);
                (key, ValueMatchPolicy::RegularExpression(value.parse()?), op)
            } else {
                let (key, op) = key_op(key);
                (key, ValueMatchPolicy::SubString(value.into()), op)
            }
        } else {
            let (key, op) = key_op(key);
            (key, ValueMatchPolicy::Exact(value.into()), op)
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

    fn match_value(&self, value: Option<&str>, escaped: bool) -> bool {
        let apply = |value| self.op.apply(self.match_policy.matches(value));
        if let Some(value) = value {
            if escaped {
                if let Some(value) = json::from_str::<&str>(value).ok() {
                    apply(value)
                } else if let Some(value) = json::from_str::<String>(value).ok() {
                    apply(&value)
                } else {
                    false
                }
            } else {
                apply(value)
            }
        } else {
            false
        }
    }

    fn match_value_partial<'a>(&self, subkey: KeyMatcher, value: RawValue<'a>) -> bool {
        if let RawValue::Object(value) = value {
            let mut item = Object::default();
            value.parse_into(&mut item).ok();
            for (k, v) in item.fields.iter() {
                match subkey.match_key(*k) {
                    None => {
                        continue;
                    }
                    Some(KeyMatch::Full) => {
                        let s = v.raw_str();
                        return self.match_value(Some(s), s.starts_with('"'));
                    }
                    Some(KeyMatch::Partial(subkey)) => {
                        return self.match_value_partial(subkey, *v);
                    }
                }
            }
        }
        false
    }
}

impl RecordFilter for FieldFilter {
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        match &self.key {
            FieldFilterKey::Predefined(kind) => match kind {
                FieldKind::Time => {
                    if let Some(ts) = &record.ts {
                        self.match_value(Some(ts.raw()), false)
                    } else {
                        false
                    }
                }
                FieldKind::Message => {
                    if let Some(message) = record.message {
                        self.match_value(Some(message.raw_str()), true)
                    } else {
                        false
                    }
                }
                FieldKind::Logger => {
                    if let Some(logger) = record.logger {
                        self.match_value(Some(logger), false)
                    } else {
                        false
                    }
                }
                FieldKind::Caller => {
                    if let Some(Caller::Text(caller)) = record.caller {
                        self.match_value(Some(caller), false)
                    } else {
                        false
                    }
                }
                _ => true,
            },
            FieldFilterKey::Custom(_) => {
                for (k, v) in record.fields_for_search() {
                    match self.match_custom_key(*k) {
                        None => {}
                        Some(KeyMatch::Full) => {
                            let s = v.raw_str();
                            let escaped = s.starts_with('"');
                            if self.match_value(Some(s), escaped) {
                                return true;
                            }
                        }
                        Some(KeyMatch::Partial(subkey)) => {
                            if self.match_value_partial(subkey, *v) {
                                return true;
                            }
                        }
                    }
                }
                false
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

const RECORD_EXTRA_CAPACITY: usize = 32;
const MAX_PREDEFINED_FIELDS: usize = 8;
const RAW_RECORD_FIELDS_CAPACITY: usize = RECORD_EXTRA_CAPACITY + MAX_PREDEFINED_FIELDS;

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use chrono::TimeZone;
    use maplit::hashmap;
    use serde_logfmt::logfmt;

    use crate::settings::{Field, FieldShowOption};

    #[test]
    fn test_raw_record_parser_empty_line() {
        let parser = RawRecordParser::new();
        let stream = parser.parse(b"");
        let mut stream = match stream {
            s @ RawRecordStream::Empty => s,
            _ => panic!(),
        };

        assert_eq!(stream.next().is_none(), true);
    }

    #[test]
    fn test_raw_record_parser_empty_object() {
        let parser = RawRecordParser::new();
        let stream = parser.parse(b"{}");
        let mut stream = match stream {
            RawRecordStream::Json(s) => s,
            _ => panic!(),
        };

        let rec = stream.next().unwrap().unwrap();
        assert_eq!(rec.prefix, b"");
        assert_eq!(rec.record.fields.as_slices().0.len(), 0);
        assert_eq!(rec.record.fields.as_slices().1.len(), 0);
    }

    #[test]
    fn test_raw_record_parser_invalid_type() {
        let parser = RawRecordParser::new().format(Some(InputFormat::Json));
        let mut stream = parser.parse(b"12");
        assert!(matches!(stream.next(), Some(Err(Error::JsonParseError(_)))));
    }

    #[test]
    fn test_raw_value_auto() {
        let value = RawValue::auto("123");
        assert_eq!(value, RawValue::Number("123"));

        let value = RawValue::auto("-123");
        assert_eq!(value, RawValue::Number("-123"));

        let value = RawValue::auto("123.0");
        assert_eq!(value, RawValue::Number("123.0"));

        let value = RawValue::auto("true");
        assert_eq!(value, RawValue::Boolean(true));

        let value = RawValue::auto("false");
        assert_eq!(value, RawValue::Boolean(false));

        let value = RawValue::auto("null");
        assert_eq!(value, RawValue::Null);

        let value = RawValue::auto(r#""123""#);
        assert_eq!(value, RawValue::String(EncodedString::json(r#""123""#)));

        let value = RawValue::auto(r#"something"#);
        assert_eq!(value, RawValue::String(EncodedString::raw(r#"something"#)));
    }

    #[test]
    fn test_raw_value_is_empty() {
        let value = RawValue::Number("0");
        assert_eq!(value.is_empty(), false);

        let value = RawValue::Number("123");
        assert_eq!(value.is_empty(), false);

        let value = RawValue::String(EncodedString::raw(""));
        assert_eq!(value.is_empty(), true);

        let value = RawValue::String(EncodedString::raw("aa"));
        assert_eq!(value.is_empty(), false);

        let value = RawValue::String(EncodedString::json(r#""""#));
        assert_eq!(value.is_empty(), true);

        let value = RawValue::String(EncodedString::json(r#""aa""#));
        assert_eq!(value.is_empty(), false);

        let value = RawValue::Boolean(true);
        assert_eq!(value.is_empty(), false);

        let value = RawValue::Null;
        assert_eq!(value.is_empty(), true);

        let value = RawValue::Object(RawObject::Json(json::from_str("{}").unwrap()));
        assert_eq!(value.is_empty(), true);

        let value = RawValue::Object(RawObject::Json(json::from_str(r#"{"a":1}"#).unwrap()));
        assert_eq!(value.is_empty(), false);

        let value = RawValue::Array(RawArray::Json(json::from_str("[]").unwrap()));
        assert_eq!(value.is_empty(), true);

        let value = RawValue::Array(RawArray::Json(json::from_str(r#"[1]"#).unwrap()));
        assert_eq!(value.is_empty(), false);
    }

    #[test]
    fn test_raw_value_raw_str() {
        let value = RawValue::Number("123");
        assert_eq!(value.raw_str(), "123");

        let value = RawValue::String(EncodedString::raw("123"));
        assert_eq!(value.raw_str(), "123");

        let value = RawValue::String(EncodedString::json(r#""123""#));
        assert_eq!(value.raw_str(), r#""123""#);

        let value = RawValue::Boolean(true);
        assert_eq!(value.raw_str(), "true");

        let value = RawValue::Null;
        assert_eq!(value.raw_str(), "null");

        let value = RawValue::Object(RawObject::Json(json::from_str("{}").unwrap()));
        assert_eq!(value.raw_str(), "{}");

        let value = RawValue::Array(RawArray::Json(json::from_str("[]").unwrap()));
        assert_eq!(value.raw_str(), "[]");
    }

    #[test]
    fn test_raw_value_parse() {
        let value = RawValue::Number("123");
        assert_eq!(value.parse::<i64>().unwrap(), 123);
        assert_eq!(value.parse::<&str>().unwrap(), "123");

        let value = RawValue::String(EncodedString::raw("123"));
        assert_eq!(value.parse::<i64>().unwrap(), 123);
        assert_eq!(value.parse::<&str>().unwrap(), "123");

        let value = RawValue::String(EncodedString::json(r#""123""#));
        assert_eq!(value.parse::<&str>().unwrap(), "123");

        let value = RawValue::Boolean(true);
        assert_eq!(value.parse::<bool>().unwrap(), true);
        assert_eq!(value.parse::<&str>().unwrap(), "true");

        let value = RawValue::Boolean(false);
        assert_eq!(value.parse::<bool>().unwrap(), false);
        assert_eq!(value.parse::<&str>().unwrap(), "false");

        let value = RawValue::Null;
        assert_eq!(value.parse::<()>().unwrap(), ());

        let value = RawValue::Object(RawObject::Json(json::from_str(r#"{"a":123}"#).unwrap()));
        assert_eq!(value.parse::<HashMap<_, _>>().unwrap(), hashmap! {"a" => 123});

        let value = RawValue::Array(RawArray::Json(json::from_str("[1,42]").unwrap()));
        assert_eq!(value.parse::<Vec<i64>>().unwrap(), vec![1, 42]);
    }

    #[test]
    fn test_raw_value_object() {
        let v1 = RawObject::Json(json::from_str(r#"{"a":123}"#).unwrap());
        let v2 = RawObject::Json(json::from_str(r#"{"a":42}"#).unwrap());
        assert_eq!(RawValue::Object(v1), RawValue::Object(v1));
        assert_ne!(RawValue::Object(v1), RawValue::Object(v2));
        assert_ne!(RawValue::Object(v1), RawValue::Number("42"));
    }

    #[test]
    fn test_raw_value_array() {
        let v1 = RawArray::Json(json::from_str(r#"[42]"#).unwrap());
        let v2 = RawArray::Json(json::from_str(r#"[43]"#).unwrap());
        assert_eq!(RawValue::Array(v1), RawValue::Array(v1));
        assert_ne!(RawValue::Array(v1), RawValue::Array(v2));
        assert_ne!(RawValue::Array(v1), RawValue::Number("42"));
    }

    #[test]
    fn test_field_filter_json_str_simple() {
        let filter = FieldFilter::parse("mod=test").unwrap();
        let record = parse(r#"{"mod":"test"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":"test2"}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":"\"test\""}"#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_field_filter_json_str_empty() {
        let filter = FieldFilter::parse("mod=").unwrap();
        let record = parse(r#"{"mod":""}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":"t"}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"v":""}"#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_field_filter_json_str_quoted() {
        let filter = FieldFilter::parse(r#"mod="test""#).unwrap();
        let record = parse(r#"{"mod":"test"}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":"test2"}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":"\"test\""}"#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_field_filter_json_str_escaped() {
        let filter = FieldFilter::parse("mod=te st").unwrap();
        let record = parse(r#"{"mod":"te st"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":"test"}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":"te\u0020st"}"#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_field_filter_json_int() {
        let filter = FieldFilter::parse("v=42").unwrap();
        let record = parse(r#"{"v":42}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"v":"42"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"v":423}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"v":"423"}"#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_field_filter_json_int_escaped() {
        let filter = FieldFilter::parse("v=42").unwrap();
        let record = parse(r#"{"v":42}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"v":"4\u0032"}"#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_field_filter_logfmt_str_simple() {
        let filter = FieldFilter::parse("mod=test").unwrap();
        let record = parse("mod=test");
        assert_eq!(filter.apply(&record), true);
        let record = parse("mod=test2");
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"mod="test""#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"mod="\"test\"""#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_field_filter_logfmt_str_empty() {
        let filter = FieldFilter::parse("mod=").unwrap();
        let record = parse(r#"mod="""#);
        assert_eq!(filter.apply(&record), true);
        let record = parse("mod=t");
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_field_filter_logfmt_str_quoted() {
        let filter = FieldFilter::parse(r#"mod="test""#).unwrap();
        let record = parse("mod=test");
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"mod=test2"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"mod="test""#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"mod="\"test\"""#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_field_filter_logfmt_str_escaped() {
        let filter = FieldFilter::parse("mod=te st").unwrap();
        let record = parse(r#"mod="te st""#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"mod=test"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"mod="te\u0020st""#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_field_filter_logfmt_int() {
        let filter = FieldFilter::parse("v=42").unwrap();
        let record = parse(r#"v=42"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"v="42""#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"v=423"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"v="423""#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_field_filter_logfmt_int_escaped() {
        let filter = FieldFilter::parse("v=42").unwrap();
        let record = parse(r#"v=42"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"v="4\u0032""#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_parse_single_word() {
        let result = try_parse("test");
        assert!(result.is_err());
        assert!(matches!(
            result.err(),
            Some(Error::LogfmtParseError(logfmt::Error::ExpectedKeyValueDelimiter))
        ));
    }

    #[test]
    fn test_record_filter_empty() {
        let filter = Filter::default();
        let record = parse(r#"{"v":42}"#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_record_filter_level() {
        let filter = Filter {
            level: Some(Level::Error),
            ..Default::default()
        };
        let record = parse(r#"{"level":"error"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"level":"info"}"#);
        assert_eq!(filter.apply(&record), false);

        let filter = Level::Error;
        let record = parse(r#"{"level":"error"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"level":"info"}"#);
        assert_eq!(filter.apply(&record), false);

        let filter = Some(Level::Info);
        let record = parse(r#"{"level":"info"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"level":"error"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"level":"debug"}"#);
        assert_eq!(filter.apply(&record), false);

        let filter: Option<Level> = None;
        let record = parse(r#"{"level":"info"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"level":"error"}"#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_record_filter_since() {
        let filter = Filter {
            since: Some(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()),
            ..Default::default()
        };
        let record = parse(r#"{"ts":"2021-01-01T00:00:00Z"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"ts":"2020-01-01T00:00:00Z"}"#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_record_filter_until() {
        let filter = Filter {
            until: Some(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()),
            ..Default::default()
        };
        let record = parse(r#"{"ts":"2021-01-01T00:00:00Z"}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"ts":"2022-01-01T00:00:00Z"}"#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_record_filter_fields() {
        let filter = Filter {
            fields: FieldFilterSet::new(&["mod=test", "v=42"]).unwrap(),
            ..Default::default()
        };
        let record = parse(r#"{"mod":"test","v":42}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":"test","v":43}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":"test2","v":42}"#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_record_filter_all() {
        let filter = Filter {
            level: Some(Level::Error),
            since: Some(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()),
            until: Some(Utc.with_ymd_and_hms(2021, 1, 2, 0, 0, 0).unwrap()),
            fields: FieldFilterSet::new(&["mod=test", "v=42"]).unwrap(),
        };
        let record = parse(r#"{"level":"error","ts":"2021-01-01T00:00:00Z","mod":"test","v":42}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"level":"info","ts":"2021-01-01T00:00:00Z","mod":"test","v":42}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"level":"error","ts":"2021-01-01T00:00:00Z","mod":"test","v":43}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"level":"error","ts":"2021-01-01T00:00:00Z","mod":"test2","v":42}"#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_record_filter_or() {
        let filter = FieldFilter::parse("mod=test")
            .unwrap()
            .or(FieldFilter::parse("v=42").unwrap());
        let record = parse(r#"{"mod":"test","v":42}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":"test","v":43}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":"test2","v":42}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":"test2","v":43}"#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_record_filter_and() {
        let filter = FieldFilter::parse("mod=test")
            .unwrap()
            .and(FieldFilter::parse("v=42").unwrap());
        let record = parse(r#"{"mod":"test","v":42}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":"test","v":43}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":"test2","v":42}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":"test2","v":43}"#);
        assert_eq!(filter.apply(&record), false);
    }

    #[test]
    fn test_field_filter_key_match() {
        let filter = FieldFilter::parse("mod.test=42").unwrap();
        let record = parse(r#"{"mod.test":42}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":{"test":42}}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":{"test":43}}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":{"test":42,"test2":42}}"#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_field_filter_key_match_partial() {
        let filter = FieldFilter::parse("mod.test=42").unwrap();
        let record = parse(r#"{"mod.test":42}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod.test2":42}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":{"test":42}}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":{"test2":42}}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":{"test":42,"test2":42}}"#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_field_filter_key_match_partial_nested() {
        let filter = FieldFilter::parse("mod.test=42").unwrap();
        let record = parse(r#"{"mod":{"test":42}}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":{"test2":42}}"#);
        assert_eq!(filter.apply(&record), false);
        let record = parse(r#"{"mod":{"test":42,"test2":42}}"#);
        assert_eq!(filter.apply(&record), true);
        let record = parse(r#"{"mod":{"test":42,"test2":42,"test3":42}}"#);
        assert_eq!(filter.apply(&record), true);
    }

    #[test]
    fn test_raw_object() {
        let obj = RawObject::Json(json::from_str(r#"{"a":1,"b":2}"#).unwrap());
        let obj = obj.parse().unwrap();
        assert_eq!(obj.fields.len(), 2);
        assert_eq!(obj.fields[0].0, "a");
        assert_eq!(obj.fields[1].0, "b");
    }

    #[test]
    fn test_raw_array() {
        let arr = RawArray::Json(json::from_str(r#"[1,2]"#).unwrap());
        let arr = arr.parse::<2>().unwrap();
        assert_eq!(arr.items.len(), 2);
        assert_eq!(arr.items[0].raw_str(), "1");
        assert_eq!(arr.items[1].raw_str(), "2");
    }

    #[test]
    fn test_array_parser_invalid_type() {
        let arr = RawArray::Json(json::from_str(r#"12"#).unwrap());
        let result = arr.parse::<2>();
        assert!(matches!(result, Err(Error::JsonParseError(_))));
    }

    #[rstest]
    #[case(br#"{"some":{"deep":{"message":"test"}}}"#, Some(r#""test""#))]
    #[case(br#"{"some":{"deep":[{"message":"test"}]}}"#, None)]
    fn test_nested_predefined_fields(#[case] input: &[u8], #[case] expected: Option<&str>) {
        let predefined = PredefinedFields {
            message: Field {
                names: vec!["some.deep.message".into()],
                show: FieldShowOption::Always,
            }
            .into(),
            ..Default::default()
        };
        let settings = ParserSettings::new(&predefined, [], None);
        let parser = Parser::new(settings);

        let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
        let record = parser.parse(&record.record);
        assert_eq!(record.message.map(|x| x.raw_str()), expected);
    }

    #[rstest]
    #[case(br#"{"ts":""}"#, None)]
    #[case(br#"{"ts":"3"}"#, Some("3"))]
    #[case(br#"ts="""#, None)]
    #[case(br#"ts="#, None)]
    #[case(br#"ts=1"#, Some("1"))]
    #[case(br#"ts="2""#, Some("2"))]
    fn test_timestamp(#[case] input: &[u8], #[case] expected: Option<&str>) {
        let parser = Parser::new(ParserSettings::default());
        let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
        let record = parser.parse(&record.record);
        assert_eq!(record.ts.map(|x| x.raw()), expected);
    }

    #[rstest]
    #[case(br#"{"level":""}"#, None)]
    #[case(br#"{"level":"info"}"#, Some(Level::Info))]
    #[case(br#"level="""#, None)]
    #[case(br#"level="#, None)]
    #[case(br#"level=info"#, Some(Level::Info))]
    #[case(br#"level="info""#, Some(Level::Info))]
    fn test_level(#[case] input: &[u8], #[case] expected: Option<Level>) {
        let parser = Parser::new(ParserSettings::default());
        let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
        let record = parser.parse(&record.record);
        assert_eq!(record.level, expected);
    }

    #[rstest]
    #[case(br#"{"logger":""}"#, None)]
    #[case(br#"{"logger":"x"}"#, Some("x"))]
    #[case(br#"logger="""#, None)]
    #[case(br#"logger="#, None)]
    #[case(br#"logger=x"#, Some("x"))]
    #[case(br#"logger="x""#, Some("x"))]
    fn test_logger(#[case] input: &[u8], #[case] expected: Option<&str>) {
        let parser = Parser::new(ParserSettings::default());
        let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
        let record = parser.parse(&record.record);
        assert_eq!(record.logger, expected);
    }

    #[rstest]
    #[case(br#"{"caller":""}"#, None)]
    #[case(br#"{"caller":"x"}"#, Some(Caller::Text("x")))]
    #[case(br#"caller="""#, None)]
    #[case(br#"caller="#, None)]
    #[case(br#"caller=x"#, Some(Caller::Text("x")))]
    #[case(br#"caller="x""#, Some(Caller::Text("x")))]
    fn test_caller(#[case] input: &[u8], #[case] expected: Option<Caller>) {
        let parser = Parser::new(ParserSettings::default());
        let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
        let record = parser.parse(&record.record);
        assert_eq!(record.caller, expected);
    }

    #[rstest]
    #[case(br#"{"file":""}"#, None)] // 1
    #[case(br#"{"file":"x"}"#, Some(Caller::FileLine("x", "")))] // 2
    #[case(br#"file="""#, None)] // 3
    #[case(br#"file="#, None)] // 4
    #[case(br#"file=x"#, Some(Caller::FileLine("x", "")))] // 5
    #[case(br#"file="x""#, Some(Caller::FileLine("x", "")))] // 6
    #[case(br#"{"line":""}"#, None)] // 7
    #[case(br#"{"line":"8"}"#, Some(Caller::FileLine("", "8")))] // 8
    #[case(br#"line="""#, None)] // 9
    #[case(br#"line="#, None)] // 10
    #[case(br#"line=11"#, Some(Caller::FileLine("", "11")))] // 11
    #[case(br#"line="12""#, Some(Caller::FileLine("", "12")))] // 12
    #[case(br#"{"file":"","line":""}"#, None)] // 13
    #[case(br#"{"file":"x","line":"14"}"#, Some(Caller::FileLine("x", "14")))] // 14
    #[case(br#"file="" line="""#, None)] // 15
    #[case(br#"file= line="#, None)] // 16
    #[case(br#"file=x line=17"#, Some(Caller::FileLine("x", "17")))] // 17
    #[case(br#"file="x" line="18""#, Some(Caller::FileLine("x", "18")))] // 18
    #[case(br#"{"file":"","line":"19"}"#, Some(Caller::FileLine("", "19")))] // 19
    #[case(br#"{"file":"x","line":""}"#, Some(Caller::FileLine("x", "")))] // 20
    #[case(br#"file="" line="21""#, Some(Caller::FileLine("", "21")))] // 21
    #[case(br#"file= line=22"#, Some(Caller::FileLine("", "22")))] // 22
    #[case(br#"file=x line="#, Some(Caller::FileLine("x", "")))] // 23
    #[case(br#"file="x" line="#, Some(Caller::FileLine("x", "")))] // 24
    #[case(br#"file="x" line=21 line=25"#, Some(Caller::FileLine("x", "25")))] // 25
    #[case(br#"file=x line=26 file=y"#, Some(Caller::FileLine("y", "26")))] // 26
    #[case(br#"{"file":123, "file": {}, "line":27}"#, Some(Caller::FileLine("123", "27")))] // 27
    #[case(br#"{"caller":"a", "file": "b", "line":28}"#, Some(Caller::Text("a")))] // 28
    #[case(br#"{"file": "b", "line":{}}"#, Some(Caller::FileLine("b", "")))] // 29
    fn test_caller_file_line(#[case] input: &[u8], #[case] expected: Option<Caller>) {
        let mut predefined = PredefinedFields::default();
        predefined.caller_file = Field {
            names: vec!["file".into()],
            show: FieldShowOption::Always,
        }
        .into();
        predefined.caller_line = Field {
            names: vec!["line".into()],
            show: FieldShowOption::Always,
        }
        .into();
        let parser = Parser::new(ParserSettings::new(&predefined, [], None));
        let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
        let record = parser.parse(&record.record);
        assert_eq!(record.caller, expected);
    }

    fn parse(s: &str) -> Record {
        try_parse(s).unwrap()
    }

    fn try_parse(s: &str) -> Result<Record> {
        let items = RawRecord::parser().parse(s.as_bytes()).collect_vec();
        assert_eq!(items.len(), 1);
        let raw = items.into_iter().next().unwrap()?.record;
        let parser = Parser::new(ParserSettings::default());
        Ok(parser.parse(&raw))
    }
}
