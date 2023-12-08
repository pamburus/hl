// std imports
use std::collections::HashMap;
use std::fmt;
use std::iter::IntoIterator;
use std::marker::PhantomData;

// third-party imports
use chrono::{DateTime, Utc};
use json::value::RawValue;
use regex::Regex;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json as json;
use wildmatch::WildMatch;

// local imports
use crate::error::{Error, Result};
use crate::level;
use crate::settings::PredefinedFields;
use crate::timestamp::Timestamp;
use crate::types::FieldKind;

// ---

pub use level::Level;

// ---

pub struct Record<'a> {
    pub ts: Option<Timestamp<'a>>,
    pub message: Option<&'a RawValue>,
    pub level: Option<Level>,
    pub logger: Option<&'a str>,
    pub caller: Option<Caller<'a>>,
    pub(crate) extra: heapless::Vec<(&'a str, &'a RawValue), RECORD_EXTRA_CAPACITY>,
    pub(crate) extrax: Vec<(&'a str, &'a RawValue)>,
}

impl<'a> Record<'a> {
    pub fn fields(&self) -> impl Iterator<Item = &(&'a str, &'a RawValue)> {
        self.extra.iter().chain(self.extrax.iter())
    }

    pub fn matches<F: RecordFilter>(&self, filter: &F) -> bool {
        filter.apply(self)
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            ts: None,
            message: None,
            level: None,
            logger: None,
            caller: None,
            extra: heapless::Vec::new(),
            extrax: if capacity > RECORD_EXTRA_CAPACITY {
                Vec::with_capacity(capacity - RECORD_EXTRA_CAPACITY)
            } else {
                Vec::new()
            },
        }
    }
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
    pub fn new(record: &'a Record<'a>, source: &'a [u8]) -> Self {
        Self { record, source }
    }
}

impl RecordWithSourceConstructor for Record<'_> {
    fn with_source<'a>(&'a self, source: &'a [u8]) -> RecordWithSource<'a> {
        RecordWithSource::new(self, source)
    }
}

// ---

pub trait RecordFilter {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool;
}

// ---

#[derive(Default, Debug)]
pub struct ParserSettings {
    pre_parse_time: bool,
    level: Vec<HashMap<String, Level>>,
    blocks: Vec<ParserSettingsBlock>,
    ignore: Vec<WildMatch>,
}

impl ParserSettings {
    pub fn new<'a, I: IntoIterator<Item = &'a String>>(
        predefined: &PredefinedFields,
        ignore: I,
        pre_parse_time: bool,
    ) -> Self {
        let mut result = Self {
            pre_parse_time,
            level: Vec::new(),
            blocks: vec![ParserSettingsBlock::default()],
            ignore: ignore.into_iter().map(|x| WildMatch::new(x)).collect(),
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
            self.level.push(mapping.clone());
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

    fn apply<'a>(&self, key: &'a str, value: &'a RawValue, to: &mut Record<'a>, pc: &mut PriorityController) {
        self.blocks[0].apply(self, key, value, to, pc, true);
    }

    fn apply_each<'a, 'i, I>(&self, items: I, to: &mut Record<'a>)
    where
        I: IntoIterator<Item = &'i (&'a str, &'a RawValue)>,
        'a: 'i,
    {
        let mut pc = PriorityController::default();
        self.apply_each_ctx(items, to, &mut pc);
    }

    fn apply_each_ctx<'a, 'i, I>(&self, items: I, to: &mut Record<'a>, pc: &mut PriorityController)
    where
        I: IntoIterator<Item = &'i (&'a str, &'a RawValue)>,
        'a: 'i,
    {
        for (key, value) in items {
            self.apply(key, value, to, pc)
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
        value: &'a RawValue,
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
        if done || !is_root {
            return;
        }

        for pattern in &ps.ignore {
            if pattern.matches(key) {
                return;
            }
        }
        match to.extra.push((key, value)) {
            Ok(_) => {}
            Err(value) => to.extrax.push(value),
        }
    }

    fn apply_each_ctx<'a, 'i, I>(
        &self,
        ps: &ParserSettings,
        items: I,
        to: &mut Record<'a>,
        ctx: &mut PriorityController,
        is_root: bool,
    ) where
        I: IntoIterator<Item = &'i (&'a str, &'a RawValue)>,
        'a: 'i,
    {
        for (key, value) in items {
            self.apply(ps, key, value, to, ctx, is_root)
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
    fn prioritize<F: FnOnce(&mut Self) -> ()>(&mut self, kind: FieldKind, priority: usize, update: F) -> bool {
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
            update(self);
            true
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
    fn apply<'a>(&self, ps: &ParserSettings, value: &'a RawValue, to: &mut Record<'a>) {
        match *self {
            Self::Time => {
                let s = value.get();
                let s = if s.as_bytes()[0] == b'"' { &s[1..s.len() - 1] } else { s };
                let ts = Timestamp::new(s, None);
                if ps.pre_parse_time {
                    to.ts = Some(Timestamp::new(ts.raw(), Some(ts.parse())));
                } else {
                    to.ts = Some(ts);
                }
            }
            Self::Level(i) => {
                to.level = json::from_str(value.get())
                    .ok()
                    .and_then(|x: &'a str| ps.level[i].get(x).cloned());
            }
            Self::Logger => to.logger = json::from_str(value.get()).ok(),
            Self::Message => to.message = Some(value),
            Self::Caller => to.caller = json::from_str(value.get()).ok().map(|x| Caller::Text(x)),
            Self::CallerFile => match &mut to.caller {
                None => {
                    to.caller = json::from_str(value.get()).ok().map(|x| Caller::FileLine(x, ""));
                }
                Some(Caller::FileLine(file, _)) => {
                    if let Some(value) = json::from_str(value.get()).ok() {
                        *file = value
                    }
                }
                _ => {}
            },
            Self::CallerLine => match &mut to.caller {
                None => {
                    to.caller = Some(Caller::FileLine("", value.get()));
                }
                Some(Caller::FileLine(_, line)) => {
                    if let Some(value) = json::from_str(value.get()).ok() {
                        *line = value
                    }
                }
                _ => {}
            },
            Self::Nested(_) => {}
        }
    }

    fn apply_ctx<'a>(
        &self,
        ps: &ParserSettings,
        value: &'a RawValue,
        to: &mut Record<'a>,
        ctx: &mut PriorityController,
    ) {
        match *self {
            Self::Nested(nested) => {
                let s = value.get();
                if s.len() > 0 && s.as_bytes()[0] == b'{' {
                    if let Ok(record) = json::from_str::<RawRecord>(s) {
                        ps.blocks[nested].apply_each_ctx(ps, record.fields(), to, ctx, false);
                    }
                }
            }
            _ => self.apply(ps, value, to),
        }
    }

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
    pub fn new(settings: ParserSettings) -> Self {
        Self { settings }
    }

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
    fields: heapless::Vec<(&'a str, &'a RawValue), RAW_RECORD_FIELDS_CAPACITY>,
    fieldsx: Vec<(&'a str, &'a RawValue)>,
}

impl<'a> RawRecord<'a> {
    pub fn fields(&self) -> impl Iterator<Item = &(&'a str, &'a RawValue)> {
        self.fields.iter().chain(self.fieldsx.iter())
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawRecord<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_map(RawRecordVisitor::new())?)
    }
}

// ---

struct RawRecordVisitor<'a> {
    marker: PhantomData<fn() -> RawRecord<'a>>,
}

impl<'a> RawRecordVisitor<'a> {
    fn new() -> Self {
        Self { marker: PhantomData }
    }
}

impl<'de: 'a, 'a> Visitor<'de> for RawRecordVisitor<'a> {
    type Value = RawRecord<'a>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object json")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> std::result::Result<Self::Value, M::Error> {
        let mut fields = heapless::Vec::new();
        let count = access.size_hint().unwrap_or(0);
        let mut fieldsx = match count > RAW_RECORD_FIELDS_CAPACITY {
            false => Vec::new(),
            true => Vec::with_capacity(count - RAW_RECORD_FIELDS_CAPACITY),
        };
        while let Some(Some(key)) = access.next_key::<&'a str>().ok() {
            match fields.push((key, access.next_value()?)) {
                Ok(_) => {}
                Err(value) => fieldsx.push(value),
            }
        }

        Ok(RawRecord { fields, fieldsx })
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
pub enum ValueMatchPolicy {
    Exact(String),
    SubString(String),
    RegularExpression(Regex),
}

impl ValueMatchPolicy {
    fn matches(&self, subject: &str) -> bool {
        match self {
            Self::Exact(pattern) => subject == pattern,
            Self::SubString(pattern) => subject.contains(pattern),
            Self::RegularExpression(pattern) => pattern.is_match(subject),
        }
    }
}

// ---

#[derive(Copy, Clone, Debug)]
enum UnaryBoolOp {
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
    fn default() -> Self {
        Self::None
    }
}

// ---

#[derive(Debug)]
pub struct FieldFilter {
    key: String,
    match_policy: ValueMatchPolicy,
    op: UnaryBoolOp,
    flat_key: bool,
}

impl FieldFilter {
    fn parse(text: &str) -> Result<Self> {
        let parse = |key, value| {
            let (key, match_policy, op) = Self::parse_mp_op(key, value)?;
            let flat_key = key.as_bytes().iter().position(|&x| x == b'.').is_none();
            Ok(Self {
                key: key.chars().map(KeyMatcher::norm).collect(),
                match_policy,
                op,
                flat_key,
            })
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

    fn match_key<'a>(&'a self, key: &str) -> Option<KeyMatch<'a>> {
        if self.flat_key && self.key.len() != key.len() {
            return None;
        }

        KeyMatcher::new(&self.key).match_key(key)
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

    fn match_value_partial(&self, subkey: KeyMatcher, value: &RawValue) -> bool {
        let bytes = value.get().as_bytes();
        if bytes[0] != b'{' {
            return false;
        }

        let item = json::from_str::<Object>(value.get()).unwrap();
        for (k, v) in item.fields.iter() {
            match subkey.match_key(*k) {
                None => {
                    continue;
                }
                Some(KeyMatch::Full) => {
                    return self.match_value(Some(v.get()), v.get().starts_with('"'));
                }
                Some(KeyMatch::Partial(subkey)) => {
                    return self.match_value_partial(subkey, *v);
                }
            }
        }
        false
    }
}

impl RecordFilter for FieldFilter {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        match &self.key[..] {
            "msg" | "message" => {
                if !self.match_value(record.message.map(|x| x.get()), true) {
                    return false;
                }
            }
            "logger" => {
                if !self.match_value(record.logger, false) {
                    return false;
                }
            }
            "caller" => {
                if let Some(Caller::Text(caller)) = record.caller {
                    if !self.match_value(Some(caller), false) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            _ => {
                let mut matched = false;
                for (k, v) in record.fields() {
                    match self.match_key(*k) {
                        None => {}
                        Some(KeyMatch::Full) => {
                            let escaped = v.get().starts_with('"');
                            matched |= self.match_value(Some(v.get()), escaped);
                        }
                        Some(KeyMatch::Partial(subkey)) => {
                            matched |= self.match_value_partial(subkey, *v);
                        }
                    }
                }
                if !matched {
                    return false;
                }
            }
        }

        true
    }
}

// ---

#[derive(Debug, Default)]
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
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        self.0.iter().all(|field| field.apply(record))
    }
}

// ---

#[derive(Debug, Default)]
pub struct Filter {
    pub fields: FieldFilterSet,
    pub level: Option<Level>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self.fields.0.is_empty() && self.level.is_none() && self.since.is_none() && self.until.is_none()
    }
}

impl RecordFilter for Filter {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        if self.is_empty() {
            return true;
        }

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
    pub fields: heapless::Vec<(&'a str, &'a RawValue), 32>,
}

struct ObjectVisitor<'a> {
    marker: PhantomData<fn() -> Object<'a>>,
}
impl<'a> ObjectVisitor<'a> {
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
            let value = access.next_value()?;
            fields.push((key, value)).ok();
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
    items: heapless::Vec<&'a RawValue, N>,
    more: Vec<&'a RawValue>,
}

impl<'a, const N: usize> Array<'a, N> {
    pub fn iter(&self) -> impl Iterator<Item = &&'a RawValue> {
        self.items.iter().chain(self.more.iter())
    }
}

struct ArrayVisitor<'a, const N: usize> {
    marker: PhantomData<fn() -> Array<'a, N>>,
}
impl<'a, const N: usize> ArrayVisitor<'a, N> {
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
        while let Some(item) = access.next_element()? {
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

const RECORD_EXTRA_CAPACITY: usize = 32;
const RAW_RECORD_FIELDS_CAPACITY: usize = RECORD_EXTRA_CAPACITY + 8;
