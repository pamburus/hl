// std imports
use std::fmt;
use std::marker::PhantomData;

// third-party imports
use heapless::consts::*;
use json::value::RawValue;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json as json;

// local imports
use crate::timestamp::Timestamp;
use crate::types;

// ---

pub use types::Level;

// ---

pub struct Record<'a> {
    ts: Option<&'a RawValue>,
    pub message: Option<&'a RawValue>,
    pub level: Option<Level>,
    pub logger: Option<&'a str>,
    pub caller: Option<&'a str>,
    extra: heapless::Vec<(&'a str, &'a RawValue), U32>,
    extrax: Vec<(&'a str, &'a RawValue)>,
}

impl<'a> Record<'a> {
    pub fn fields(&self) -> impl Iterator<Item = &(&'a str, &'a RawValue)> {
        self.extra.iter().chain(self.extrax.iter())
    }

    pub fn ts(&self) -> Option<Timestamp<'a>> {
        match self.ts {
            None => None,
            Some(ts) => {
                let s = ts.get();
                let s = if s.as_bytes()[0] == b'"' {
                    &s[1..s.len() - 1]
                } else {
                    s
                };
                Some(Timestamp::new(s))
            }
        }
    }

    pub fn matches(&self, filter: &Filter) -> bool {
        if filter.is_empty() {
            return true;
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
}

impl<'de: 'a, 'a> Deserialize<'de> for Record<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_map(RecordVisitor::new())?)
    }
}

// ---

struct RecordVisitor<'a> {
    marker: PhantomData<fn() -> Record<'a>>,
}

impl<'a> RecordVisitor<'a> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de: 'a, 'a> Visitor<'de> for RecordVisitor<'a> {
    type Value = Record<'a>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object json")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
        let mut ts = None;
        let mut rts = None;
        let mut message = None;
        let mut level = None;
        let mut priority = None;
        let mut logger = None;
        let mut caller = None;
        let mut extra = heapless::Vec::new();
        let count = access.size_hint().unwrap_or(0);
        let mut extrax = match count > 32 {
            false => Vec::new(),
            true => Vec::with_capacity(count - 32),
        };
        while let Some(Some(key)) = access.next_key::<&'a str>().ok() {
            match key {
                "ts" | "TS" | "time" | "TIME" | "Time" => {
                    ts = access.next_value()?;
                }
                "_SOURCE_REALTIME_TIMESTAMP" => {
                    rts = access.next_value()?;
                }
                "__REALTIME_TIMESTAMP" => {
                    rts = rts.or(Some(access.next_value()?));
                }
                "msg" | "message" | "MESSAGE" | "Message" => {
                    message = access.next_value()?;
                }
                "level" | "LEVEL" | "Level" => {
                    level = access.next_value()?;
                }
                "PRIORITY" => {
                    priority = access.next_value()?;
                }
                "logger" | "LOGGER" | "Logger" => {
                    logger = access.next_value()?;
                }
                "caller" | "CALLER" | "Caller" => {
                    caller = access.next_value()?;
                }
                _ => {
                    if key.starts_with("_") {
                        drop(access.next_value::<&'a RawValue>().ok());
                        continue;
                    }
                    match extra.push((key, access.next_value()?)) {
                        Ok(_) => {}
                        Err(value) => extrax.push(value),
                    }
                }
            }
        }

        let level = match level {
            Some("debug") => Some(Level::Debug),
            Some("info") | Some("information") => Some(Level::Info),
            Some("warn") | Some("warning") => Some(Level::Warning),
            Some("err") | Some("error") | Some("fatal") | Some("panic") => Some(Level::Error),
            _ => match priority {
                Some("7") => Some(Level::Debug),
                Some("6") => Some(Level::Info),
                Some("5") | Some("4") => Some(Level::Warning),
                Some("3") | Some("2") | Some("1") => Some(Level::Error),
                _ => None,
            },
        };
        Ok(Record {
            ts: ts.or(rts),
            message,
            level,
            logger,
            caller,
            extra,
            extrax,
        })
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Filter {
    pub fields: FieldFilterSet,
    pub level: Option<Level>,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self.fields.0.is_empty() && self.level.is_none()
    }
}

pub struct Object<'a> {
    pub fields: heapless::Vec<(&'a str, &'a RawValue), U32>,
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

pub struct Array<'a, N>
where
    N: heapless::ArrayLength<&'a RawValue>,
{
    items: heapless::Vec<&'a RawValue, N>,
    more: Vec<&'a RawValue>,
}

impl<'a, N> Array<'a, N>
where
    N: heapless::ArrayLength<&'a RawValue>,
{
    pub fn iter(&self) -> impl Iterator<Item = &&'a RawValue> {
        self.items.iter().chain(self.more.iter())
    }
}

struct ArrayVisitor<'a, N>
where
    N: heapless::ArrayLength<&'a RawValue>,
{
    marker: PhantomData<fn() -> Array<'a, N>>,
}
impl<'a, N> ArrayVisitor<'a, N>
where
    N: heapless::ArrayLength<&'a RawValue>,
{
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de: 'a, 'a, N> Visitor<'de> for ArrayVisitor<'a, N>
where
    N: heapless::ArrayLength<&'a RawValue>,
{
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

impl<'de: 'a, 'a, N> Deserialize<'de> for Array<'a, N>
where
    N: heapless::ArrayLength<&'a RawValue>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_seq(ArrayVisitor::new())?)
    }
}
