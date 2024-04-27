// std imports
use std::{
    cmp::Ordering, collections::HashMap, fmt, iter::IntoIterator, marker::PhantomData, ops::Range, str::FromStr,
};

// third-party imports
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{self as json};
use wildflower::Pattern;

// local imports
use crate::{
    app::{InputFormat, UnixTimestampUnit},
    error::{Error, Result},
    level, logfmt,
    serdex::StreamDeserializerWithOffsets,
    settings::PredefinedFields,
    timestamp::Timestamp,
    types::FieldKind,
};
use encstr::{AnyEncodedString, EncodedString};

// ---

pub use level::Level;

// ---

#[derive(Clone, Copy)]
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
        let looks_like_number = || {
            let mut s = value;
            let mut n_dots = 0;
            if s.starts_with('-') {
                s = &s[1..];
            }
            s.len() < 40
                && s.as_bytes().iter().all(|&x| {
                    if x == b'.' {
                        n_dots += 1;
                        n_dots <= 1
                    } else {
                        x.is_ascii_digit()
                    }
                })
        };

        match value.as_bytes() {
            [b'"', ..] => Self::String(EncodedString::Json(value.into())),
            b"false" => Self::Boolean(false),
            b"true" => Self::Boolean(true),
            b"null" => Self::Null,
            _ if looks_like_number() => Self::Number(value),
            _ => Self::String(EncodedString::Raw(value.into())),
        }
    }

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
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
            Self::Boolean(true) => ("true", true),
            Self::Boolean(false) => ("false", true),
            Self::Number(value) => (*value, false),
        };

        if is_json {
            json::from_str(s).map_err(Error::JsonParseError)
        } else {
            logfmt::from_str(s).map_err(Error::LogfmtParseError)
        }
    }

    #[inline(always)]
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

    #[inline(always)]
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

impl<'a> From<&'a json::value::RawValue> for RawValue<'a> {
    #[inline(always)]
    fn from(value: &'a json::value::RawValue) -> Self {
        match value.get().as_bytes()[0] {
            b'"' => Self::String(EncodedString::Json(value.get().into())),
            b'0'..=b'9' | b'-' | b'+' | b'.' => Self::Number(value.get()),
            b'{' => Self::from(RawObject::Json(value)),
            b'[' => Self::from(RawArray::Json(value)),
            b't' => Self::Boolean(true),
            b'f' => Self::Boolean(false),
            b'n' => Self::Null,
            _ => Self::String(EncodedString::raw(value.get())),
        }
    }
}

impl<'a> From<&'a logfmt::raw::RawValue> for RawValue<'a> {
    #[inline(always)]
    fn from(value: &'a logfmt::raw::RawValue) -> Self {
        if value.get().as_bytes()[0] == b'"' {
            Self::String(EncodedString::Json(value.get().into()))
        } else {
            Self::String(EncodedString::Raw(value.get().into()))
        }
    }
}

impl<'a> From<RawObject<'a>> for RawValue<'a> {
    #[inline(always)]
    fn from(value: RawObject<'a>) -> Self {
        Self::Object(value)
    }
}

impl<'a> From<RawArray<'a>> for RawValue<'a> {
    #[inline(always)]
    fn from(value: RawArray<'a>) -> Self {
        Self::Array(value)
    }
}

// ---

#[derive(Clone, Copy)]
pub enum RawObject<'a> {
    Json(&'a json::value::RawValue),
}

impl<'a> RawObject<'a> {
    #[inline(always)]
    pub fn get(&self) -> &'a str {
        match self {
            Self::Json(value) => value.get(),
        }
    }

    #[inline(always)]
    pub fn parse(&self) -> Result<Object<'a>> {
        match self {
            Self::Json(value) => json::from_str::<Object>(value.get()).map_err(Error::JsonParseError),
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Json(value) => json_match(value, "{}"),
        }
    }
}

impl<'a> From<&'a json::value::RawValue> for RawObject<'a> {
    #[inline(always)]
    fn from(value: &'a json::value::RawValue) -> Self {
        Self::Json(value)
    }
}

// ---

#[derive(Clone, Copy)]
pub enum RawArray<'a> {
    Json(&'a json::value::RawValue),
}

impl<'a> RawArray<'a> {
    #[inline(always)]
    pub fn get(&self) -> &'a str {
        match self {
            Self::Json(value) => value.get(),
        }
    }

    #[inline(always)]
    pub fn parse<const N: usize>(&self) -> Result<Array<'a, N>> {
        json::from_str::<Array<N>>(self.get()).map_err(Error::JsonParseError)
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Json(value) => json_match(value, "[]"),
        }
    }
}

impl<'a> From<&'a json::value::RawValue> for RawArray<'a> {
    #[inline(always)]
    fn from(value: &'a json::value::RawValue) -> Self {
        Self::Json(value)
    }
}

// ---

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
    #[inline(always)]
    pub fn fields(&self) -> impl Iterator<Item = &(&'a str, RawValue<'a>)> {
        self.fields.head.iter().chain(self.fields.tail.iter())
    }

    #[inline(always)]
    pub fn fields_for_search(&self) -> impl Iterator<Item = &(&'a str, RawValue<'a>)> {
        self.fields().chain(self.predefined.iter())
    }

    #[inline(always)]
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
            fields: RecordFields {
                head: heapless::Vec::new(),
                tail: if capacity > RECORD_EXTRA_CAPACITY {
                    Vec::with_capacity(capacity - RECORD_EXTRA_CAPACITY)
                } else {
                    Vec::new()
                },
            },
            predefined: heapless::Vec::new(),
        }
    }
}

pub struct RecordFields<'a> {
    pub(crate) head: heapless::Vec<(&'a str, RawValue<'a>), RECORD_EXTRA_CAPACITY>,
    pub(crate) tail: Vec<(&'a str, RawValue<'a>)>,
}

// ---

pub trait RecordWithSourceConstructor {
    fn with_source<'a>(&'a self, source: &'a [u8]) -> RecordWithSource<'a>;
}

// ---

pub enum Caller<'a> {
    Text(&'a str),
    FileLine(&'a str, &'a str),
}

// ---

pub struct RecordWithSource<'a> {
    pub record: &'a Record<'a>,
    pub source: &'a [u8],
}

impl<'a> RecordWithSource<'a> {
    #[inline(always)]
    pub fn new(record: &'a Record<'a>, source: &'a [u8]) -> Self {
        Self { record, source }
    }
}

impl RecordWithSourceConstructor for Record<'_> {
    #[inline(always)]
    fn with_source<'a>(&'a self, source: &'a [u8]) -> RecordWithSource<'a> {
        RecordWithSource::new(self, source)
    }
}

// ---

pub trait RecordFilter {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool;

    #[inline(always)]
    fn and<F>(self, rhs: F) -> RecordFilterAnd<Self, F>
    where
        Self: Sized,
        F: RecordFilter,
    {
        RecordFilterAnd { lhs: self, rhs }
    }

    #[inline(always)]
    fn or<F>(self, rhs: F) -> RecordFilterOr<Self, F>
    where
        Self: Sized,
        F: RecordFilter,
    {
        RecordFilterOr { lhs: self, rhs }
    }
}

impl<T: RecordFilter + ?Sized> RecordFilter for Box<T> {
    #[inline(always)]
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl<T: RecordFilter> RecordFilter for &T {
    #[inline(always)]
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl<T: RecordFilter> RecordFilter for Option<T> {
    #[inline(always)]
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
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
    #[inline(always)]
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        self.lhs.apply(record) && self.rhs.apply(record)
    }
}

// ---

pub struct RecordFilterOr<L: RecordFilter, R: RecordFilter> {
    lhs: L,
    rhs: R,
}

impl<L: RecordFilter, R: RecordFilter> RecordFilter for RecordFilterOr<L, R> {
    #[inline(always)]
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        self.lhs.apply(record) || self.rhs.apply(record)
    }
}

// ---

pub struct RecordFilterNone;

impl RecordFilter for RecordFilterNone {
    #[inline(always)]
    fn apply<'a>(&self, _: &'a Record<'a>) -> bool {
        true
    }
}

// ---

#[derive(Default)]
pub struct ParserSettings {
    pre_parse_time: bool,
    unix_ts_unit: Option<UnixTimestampUnit>,
    level: Vec<(HashMap<String, Level>, Option<Level>)>,
    blocks: Vec<ParserSettingsBlock>,
    ignore: Vec<Pattern<String>>,
}

impl ParserSettings {
    pub fn new<'a, I: IntoIterator<Item = &'a String>>(
        predefined: &PredefinedFields,
        ignore: I,
        pre_parse_time: bool,
        unix_ts_unit: Option<UnixTimestampUnit>,
    ) -> Self {
        let mut result = Self {
            pre_parse_time,
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
            let mut mapping = HashMap::new();
            for (level, values) in &variant.values {
                for value in values {
                    mapping.insert(value.clone(), level.clone());
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

    #[inline(always)]
    fn apply<'a>(&self, key: &'a str, value: RawValue<'a>, to: &mut Record<'a>, pc: &mut PriorityController) {
        self.blocks[0].apply(self, key, value, to, pc, true);
    }

    #[inline(always)]
    fn apply_each<'a, 'i, I>(&self, items: I, to: &mut Record<'a>)
    where
        I: IntoIterator<Item = &'i (&'a str, RawValue<'a>)>,
        'a: 'i,
    {
        let mut pc = PriorityController::default();
        self.apply_each_ctx(items, to, &mut pc);
    }

    #[inline(always)]
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
        match to.fields.head.push((key, value)) {
            Ok(_) => {}
            Err(value) => to.fields.tail.push(value),
        }
    }

    #[inline(always)]
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
    #[inline(always)]
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
                let s = if s.as_bytes()[0] == b'"' { &s[1..s.len() - 1] } else { s };
                let ts = Timestamp::new(s).with_unix_unit(ps.unix_ts_unit);
                if ps.pre_parse_time {
                    to.ts = Some(ts.preparsed())
                } else {
                    to.ts = Some(ts);
                }
                true
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
                to.logger = value.parse().ok();
                true
            }
            Self::Message => {
                to.message = Some(value);
                true
            }
            Self::Caller => {
                to.caller = value.parse().ok().map(|x| Caller::Text(x));
                true
            }
            Self::CallerFile => match &mut to.caller {
                None => {
                    to.caller = value.parse().ok().map(|x| Caller::FileLine(x, ""));
                    to.caller.is_some()
                }
                Some(Caller::FileLine(file, _)) => {
                    if let Some(value) = value.parse().ok() {
                        *file = value;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Self::CallerLine => match &mut to.caller {
                None => {
                    to.caller = Some(Caller::FileLine("", value.raw_str()));
                    true
                }
                Some(Caller::FileLine(_, line)) => match value {
                    RawValue::Number(value) => {
                        *line = value;
                        true
                    }
                    RawValue::String(_) => {
                        if let Some(value) = value.parse().ok() {
                            *line = value;
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                },
                _ => false,
            },
            Self::Nested(_) => false,
        }
    }

    #[inline(always)]
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
                    if let Ok(record) = json::from_str::<RawRecord>(value.get()) {
                        ps.blocks[nested].apply_each_ctx(ps, record.fields(), to, ctx, false);
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

    #[inline(always)]
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
    pub fn parse<'a>(&self, record: RawRecord<'a>) -> Record<'a> {
        let fields = record.fields();
        let count = fields.size_hint().1.unwrap_or(0);
        let mut record = Record::<'a>::with_capacity(count);

        self.settings.apply_each(fields, &mut record);

        record
    }
}

// ---

pub struct RawRecord<'a> {
    fields: RawRecordFields<'a>,
}

pub struct RawRecordFields<'a> {
    head: heapless::Vec<(&'a str, RawValue<'a>), RAW_RECORD_FIELDS_CAPACITY>,
    tail: Vec<(&'a str, RawValue<'a>)>,
}

impl<'a> RawRecord<'a> {
    #[inline]
    pub fn fields(&self) -> impl Iterator<Item = &(&'a str, RawValue<'a>)> {
        self.fields.head.iter().chain(self.fields.tail.iter())
    }

    #[inline]
    pub fn parser() -> RawRecordParser {
        RawRecordParser::new()
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawRecord<'a> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_map(RawRecordVisitor::<json::value::RawValue>::new())?)
    }
}

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
    pub fn parse<'a>(&self, line: &'a [u8]) -> RawRecordStream<impl RawRecordIterator<'a>, impl RawRecordIterator<'a>> {
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

struct RawRecordJsonStream<'a, 'de, R> {
    prefix: &'a [u8],
    delegate: StreamDeserializerWithOffsets<'de, R, RawRecord<'a>>,
}

impl<'a, 'de: 'a, R> RawRecordIterator<'a> for RawRecordJsonStream<'a, 'de, R>
where
    R: serde_json::de::Read<'de>,
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

struct RawRecordVisitor<'a, RV>
where
    RV: ?Sized + 'a,
{
    marker: PhantomData<fn() -> (RawRecord<'a>, &'a RV)>,
}

impl<'a, RV> RawRecordVisitor<'a, RV>
where
    RV: ?Sized + 'a,
{
    #[inline(always)]
    fn new() -> Self {
        Self { marker: PhantomData }
    }
}

impl<'de: 'a, 'a, RV> Visitor<'de> for RawRecordVisitor<'a, RV>
where
    RV: ?Sized + 'a,
    &'a RV: Deserialize<'de> + 'a,
    RawValue<'a>: std::convert::From<&'a RV>,
{
    type Value = RawRecord<'a>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object json")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> std::result::Result<Self::Value, M::Error> {
        let mut head = heapless::Vec::new();
        let count = access.size_hint().unwrap_or(0);
        let mut tail = match count > RAW_RECORD_FIELDS_CAPACITY {
            false => Vec::new(),
            true => Vec::with_capacity(count - RAW_RECORD_FIELDS_CAPACITY),
        };
        while let Some(Some(key)) = access.next_key::<&'a str>().ok() {
            let value: &RV = access.next_value()?;
            match head.push((key, value.into())) {
                Ok(_) => {}
                Err(value) => tail.push(value),
            }
        }

        Ok(RawRecord {
            fields: RawRecordFields { head, tail },
        })
    }
}

// ---

pub struct LogfmtRawRecord<'a>(pub RawRecord<'a>);

impl<'de: 'a, 'a> Deserialize<'de> for LogfmtRawRecord<'a> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(
            deserializer.deserialize_map(RawRecordVisitor::<logfmt::raw::RawValue>::new())?,
        ))
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
    #[inline(always)]
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

    #[inline(always)]
    fn norm(c: char) -> char {
        if c == '_' {
            '-'
        } else {
            c.to_ascii_lowercase()
        }
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
    #[inline(always)]
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
    #[inline(always)]
    fn from(value: i128) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for Number {
    #[inline(always)]
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl PartialOrd<Number> for Number {
    #[inline(always)]
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
    In(Vec<String>),
    WildCard(Pattern<String>),
    Numerically(NumericOp),
}

impl ValueMatchPolicy {
    fn matches(&self, subject: &str) -> bool {
        match self {
            Self::Exact(pattern) => subject == pattern,
            Self::SubString(pattern) => subject.contains(pattern),
            Self::RegularExpression(pattern) => pattern.is_match(subject),
            Self::In(patterns) => patterns.iter().any(|pattern| subject == pattern),
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
    #[inline(always)]
    fn apply(self, value: bool) -> bool {
        match self {
            Self::None => value,
            Self::Negate => !value,
        }
    }
}

impl Default for UnaryBoolOp {
    #[inline(always)]
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
    #[inline(always)]
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

    #[inline(always)]
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
            let item = value.parse().unwrap();
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
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
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
    #[inline(always)]
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
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
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.fields.0.is_empty() && self.level.is_none() && self.since.is_none() && self.until.is_none()
    }
}

impl RecordFilter for Filter {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
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

pub struct Object<'a> {
    pub fields: heapless::Vec<(&'a str, RawValue<'a>), 32>,
}

struct ObjectVisitor<'a> {
    marker: PhantomData<fn() -> Object<'a>>,
}
impl<'a> ObjectVisitor<'a> {
    #[inline(always)]
    fn new() -> Self {
        Self { marker: PhantomData }
    }
}

impl<'de: 'a, 'a> Visitor<'de> for ObjectVisitor<'a> {
    type Value = Object<'a>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object json")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> std::result::Result<Self::Value, A::Error> {
        let mut fields = heapless::Vec::new();
        while let Some(key) = access.next_key::<&'a str>()? {
            let value: &json::value::RawValue = access.next_value()?;
            fields.push((key, value.into())).ok();
        }

        Ok(Object { fields })
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for Object<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_map(ObjectVisitor::new())?)
    }
}

pub struct Array<'a, const N: usize> {
    items: heapless::Vec<RawValue<'a>, N>,
    more: Vec<RawValue<'a>>,
}

impl<'a, const N: usize> Array<'a, N> {
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &RawValue<'a>> {
        self.items.iter().chain(self.more.iter())
    }
}

struct ArrayVisitor<'a, const N: usize> {
    marker: PhantomData<fn() -> Array<'a, N>>,
}
impl<'a, const N: usize> ArrayVisitor<'a, N> {
    #[inline(always)]
    fn new() -> Self {
        Self { marker: PhantomData }
    }
}

impl<'de: 'a, 'a, const N: usize> Visitor<'de> for ArrayVisitor<'a, N> {
    type Value = Array<'a, N>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object json")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> std::result::Result<Self::Value, A::Error> {
        let mut items = heapless::Vec::new();
        let mut more = Vec::new();
        while let Some(item) = access.next_element::<&json::value::RawValue>()? {
            let item = item.into();
            match items.push(item) {
                Ok(()) => {}
                Err(item) => more.push(item),
            }
        }
        Ok(Array { items, more })
    }
}

impl<'de: 'a, 'a, const N: usize> Deserialize<'de> for Array<'a, N> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_seq(ArrayVisitor::new())?)
    }
}

// ---

#[inline(always)]
fn is_json_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

#[inline(always)]
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
        assert_eq!(rec.record.fields.head.len(), 0);
        assert_eq!(rec.record.fields.tail.len(), 0);
    }
}
