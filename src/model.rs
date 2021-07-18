// std imports
use std::collections::HashMap;
use std::fmt;
use std::iter::IntoIterator;
use std::marker::PhantomData;

// third-party imports
use chrono::{DateTime, Utc};
use json::value::RawValue;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json as json;
use wildmatch::WildMatch;

// local imports
use crate::settings::Fields;
use crate::timestamp::Timestamp;
use crate::types::{self, FieldKind};

// ---

pub use types::Level;

// ---

pub struct Record<'a> {
    pub ts: Option<Timestamp<'a>>,
    pub message: Option<&'a RawValue>,
    pub level: Option<Level>,
    pub logger: Option<&'a str>,
    pub caller: Option<&'a str>,
    extra: heapless::Vec<(&'a str, &'a RawValue), RECORD_EXTRA_CAPACITY>,
    extrax: Vec<(&'a str, &'a RawValue)>,
}

impl<'a> Record<'a> {
    pub fn fields(&self) -> impl Iterator<Item = &(&'a str, &'a RawValue)> {
        self.extra.iter().chain(self.extrax.iter())
    }

    pub fn matches(&self, filter: &Filter) -> bool {
        if filter.is_empty() {
            return true;
        }

        if filter.since.is_some() || filter.until.is_some() {
            if let Some(ts) = self.ts.as_ref().and_then(|ts| ts.parse()) {
                if let Some(since) = filter.since {
                    if ts < since {
                        return false;
                    }
                }
                if let Some(until) = filter.until {
                    if ts > until {
                        return false;
                    }
                }
            }
        }

        if let Some(bound) = &filter.level {
            if let Some(level) = self.level.as_ref() {
                if level > bound {
                    return false;
                }
            }
        }

        if !filter.fields.0.is_empty() {
            for field in filter.fields.0.iter() {
                match &field.key[..] {
                    "msg" | "message" => {
                        if !field.match_value(self.message.map(|x| x.get()), true) {
                            return false;
                        }
                    }
                    "logger" => {
                        if !field.match_value(self.logger, false) {
                            return false;
                        }
                    }
                    "caller" => {
                        if !field.match_value(self.caller, false) {
                            return false;
                        }
                    }
                    _ => {
                        let mut matched = false;
                        for (k, v) in self.extra.iter() {
                            match field.match_key(*k) {
                                None => {}
                                Some(KeyMatch::Full) => {
                                    let escaped = v.get().starts_with('"');
                                    matched |= field.match_value(Some(v.get()), escaped);
                                }
                                Some(KeyMatch::Partial(subkey)) => {
                                    matched |= field.match_value_partial(subkey, *v);
                                }
                            }
                        }
                        if !matched {
                            return false;
                        }
                    }
                }
            }
        }

        return true;
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

#[derive(Default)]
pub struct ParserSettings {
    fields: HashMap<String, (FieldSettings, usize)>,
    ignore: Vec<WildMatch>,
}

impl ParserSettings {
    pub fn new(s: &Fields, preparse_time: bool) -> Self {
        let mut fields = HashMap::new();
        for (i, name) in s.predefined.time.names.iter().enumerate() {
            fields.insert(name.clone(), (FieldSettings::Time(preparse_time), i));
        }
        let mut j = 0;
        for variant in &s.predefined.level.variants {
            let mut mapping = HashMap::new();
            for (level, values) in &variant.values {
                for value in values {
                    mapping.insert(value.clone(), level.clone());
                }
            }
            for (i, name) in variant.names.iter().enumerate() {
                fields.insert(name.clone(), (FieldSettings::Level(mapping.clone()), j + i));
            }
            j += variant.names.len();
        }
        for (i, name) in s.predefined.message.names.iter().enumerate() {
            fields.insert(name.clone(), (FieldSettings::Message, i));
        }
        for (i, name) in s.predefined.logger.names.iter().enumerate() {
            fields.insert(name.clone(), (FieldSettings::Logger, i));
        }
        for (i, name) in s.predefined.caller.names.iter().enumerate() {
            fields.insert(name.clone(), (FieldSettings::Caller, i));
        }
        Self {
            fields,
            ignore: s.ignore.iter().map(|v| WildMatch::new(v)).collect(),
        }
    }

    fn apply<'a>(
        &self,
        key: &'a str,
        value: &'a RawValue,
        to: &mut Record<'a>,
        ctx: &mut PriorityContext,
    ) {
        match self.fields.get(key) {
            Some((field, p)) => {
                let kind = field.kind();
                let priority = ctx.priority(kind);
                if priority.is_none() || Some(*p) <= *priority {
                    field.apply(value, to);
                    *priority = Some(*p);
                }
            }
            None => {
                for pattern in &self.ignore {
                    if pattern.matches(key) {
                        return;
                    }
                }
                match to.extra.push((key, value)) {
                    Ok(_) => {}
                    Err(value) => to.extrax.push(value),
                }
            }
        };
    }

    fn apply_each<'a, 'i, I>(&self, items: I, to: &mut Record<'a>)
    where
        I: IntoIterator<Item = &'i (&'a str, &'a RawValue)>,
        'a: 'i,
    {
        let mut ctx = PriorityContext {
            time: None,
            level: None,
            logger: None,
            message: None,
            caller: None,
        };
        for (key, value) in items {
            self.apply(key, value, to, &mut ctx)
        }
    }
}

// ---

struct PriorityContext {
    time: Option<usize>,
    level: Option<usize>,
    logger: Option<usize>,
    message: Option<usize>,
    caller: Option<usize>,
}

impl PriorityContext {
    fn priority(&mut self, kind: FieldKind) -> &mut Option<usize> {
        match kind {
            FieldKind::Time => &mut self.time,
            FieldKind::Level => &mut self.level,
            FieldKind::Logger => &mut self.logger,
            FieldKind::Message => &mut self.message,
            FieldKind::Caller => &mut self.caller,
        }
    }
}

// ---

enum FieldSettings {
    Time(bool),
    Level(HashMap<String, Level>),
    Logger,
    Message,
    Caller,
}

impl FieldSettings {
    fn apply<'a>(&self, value: &'a RawValue, to: &mut Record<'a>) {
        match self {
            Self::Time(preparse) => {
                let s = value.get();
                let s = if s.as_bytes()[0] == b'"' {
                    &s[1..s.len() - 1]
                } else {
                    s
                };
                let ts = Timestamp::new(s, None);
                if *preparse {
                    to.ts = Some(Timestamp::new(ts.raw(), Some(ts.parse())));
                } else {
                    to.ts = Some(ts);
                }
            }
            Self::Level(values) => {
                to.level = json::from_str(value.get())
                    .ok()
                    .and_then(|x: &'a str| values.get(x).cloned());
            }
            Self::Logger => to.logger = json::from_str(value.get()).ok(),
            Self::Message => to.message = Some(value),
            Self::Caller => to.caller = json::from_str(value.get()).ok(),
        }
    }

    fn kind(&self) -> FieldKind {
        match self {
            Self::Time(_) => FieldKind::Time,
            Self::Level(_) => FieldKind::Level,
            Self::Logger => FieldKind::Logger,
            Self::Message => FieldKind::Message,
            Self::Caller => FieldKind::Caller,
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de: 'a, 'a> Visitor<'de> for RawRecordVisitor<'a> {
    type Value = RawRecord<'a>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object json")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
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
enum Operator {
    Equal,
    NotEqual,
    Like,
    NotLike,
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
        let norm = |b: u8| {
            if b == b'_' {
                b'-'
            } else {
                b.to_ascii_lowercase()
            }
        };
        let bytes = self.key.as_bytes();
        if bytes
            .iter()
            .zip(key.as_bytes().iter())
            .position(|(&x, &y)| norm(x) != norm(y))
            .is_some()
        {
            return None;
        }

        if self.key.len() == key.len() {
            Some(KeyMatch::Full)
        } else if self.key.len() > key.len() {
            if bytes[key.len()] == b'.' {
                Some(KeyMatch::Partial(KeyMatcher::new(
                    &self.key[key.len() + 1..],
                )))
            } else {
                None
            }
        } else {
            None
        }
    }
}

// ---

#[derive(Debug)]
pub struct FieldFilter {
    key: String,
    value: String,
    value_escaped: String,
    operator: Operator,
    flat_key: bool,
}

impl FieldFilter {
    fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('=');
        match (parts.next(), parts.next()) {
            (Some(mut key), Some(value)) => {
                let operator = if key.ends_with('~') {
                    key = &key[..key.len() - 1];
                    if key.ends_with('!') {
                        key = &key[..key.len() - 1];
                        Operator::NotLike
                    } else {
                        Operator::Like
                    }
                } else {
                    if key.ends_with('!') {
                        key = &key[..key.len() - 1];
                        Operator::NotEqual
                    } else {
                        Operator::Equal
                    }
                };
                let flat_key = key.as_bytes().iter().position(|&x| x == b'.').is_none();
                Some(Self {
                    key: key.into(),
                    value: value.into(),
                    value_escaped: json::to_string(value).unwrap(),
                    operator: operator,
                    flat_key,
                })
            }
            _ => None,
        }
    }

    fn match_key<'a>(&'a self, key: &str) -> Option<KeyMatch<'a>> {
        if self.flat_key && self.key.len() != key.len() {
            return None;
        }

        KeyMatcher::new(&self.key).match_key(key)
    }

    fn match_value(&self, value: Option<&str>, escaped: bool) -> bool {
        let pattern = if escaped {
            &self.value_escaped
        } else {
            &self.value
        };

        match (&self.operator, value) {
            (Operator::Equal, Some(value)) => pattern == value,
            (Operator::NotEqual, Some(value)) => pattern != value,
            (Operator::Like, Some(value)) => value.contains(&self.value[..]),
            (Operator::NotLike, Some(value)) => !value.contains(&self.value[..]),
            (_, None) => false,
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

// ---

#[derive(Debug, Default)]
pub struct FieldFilterSet(Vec<FieldFilter>);

impl FieldFilterSet {
    pub fn new<T: AsRef<str>, I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut fields = Vec::new();
        for i in items {
            if let Some(item) = FieldFilter::parse(i.as_ref()) {
                fields.push(item);
            }
        }
        FieldFilterSet(fields)
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
        self.fields.0.is_empty()
            && self.level.is_none()
            && self.since.is_none()
            && self.until.is_none()
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
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de: 'a, 'a> Visitor<'de> for ObjectVisitor<'a> {
    type Value = Object<'a>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object json")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut fields = heapless::Vec::new();
        while let Some(key) = access.next_key::<&'a str>()? {
            let value = access.next_value()?;
            fields.push((key, value)).ok();
        }

        Ok(Object { fields })
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for Object<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de: 'a, 'a, const N: usize> Visitor<'de> for ArrayVisitor<'a, N> {
    type Value = Array<'a, N>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object json")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_seq(ArrayVisitor::new())?)
    }
}

// ---

const RECORD_EXTRA_CAPACITY: usize = 32;
const RAW_RECORD_FIELDS_CAPACITY: usize = RECORD_EXTRA_CAPACITY + 8;
