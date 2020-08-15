use std::fmt;
use std::marker::PhantomData;

use heapless::consts::*;
use json::value::RawValue;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json as json;

use crate::timestamp::Timestamp;
use crate::types;

pub use types::Level;

pub struct Message<'a> {
    ts: Option<&'a RawValue>,
    pub text: Option<&'a RawValue>,
    pub level: Option<Level>,
    pub logger: Option<&'a str>,
    pub caller: Option<&'a str>,
    extra: heapless::Vec<(&'a str, &'a RawValue), U32>,
    extrax: Vec<(&'a str, &'a RawValue)>,
}

impl<'a> Message<'a> {
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
                        if !field.value_match(self.text.map(|x| x.get()), true) {
                            return false;
                        }
                    }
                    "logger" => {
                        if !field.value_match(self.logger, false) {
                            return false;
                        }
                    }
                    "caller" => {
                        if !field.value_match(self.caller, false) {
                            return false;
                        }
                    }
                    _ => {
                        let mut found = false;
                        for (k, v) in self.extra.iter() {
                            if field.key_match(*k) {
                                found = true;
                                if !field.value_match(Some(v.get()), v.get().starts_with('"')) {
                                    return false;
                                }
                            }
                        }
                        if !found {
                            return false;
                        }
                    }
                }
            }
        }

        return true;
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for Message<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_map(MessageVisitor::new())?)
    }
}

struct MessageVisitor<'a> {
    marker: PhantomData<fn() -> Message<'a>>,
}

impl<'a> MessageVisitor<'a> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de: 'a, 'a> Visitor<'de> for MessageVisitor<'a> {
    type Value = Message<'a>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object json")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
        let mut ts = None;
        let mut rts = None;
        let mut text = None;
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
        while let Some(key) = access.next_key::<&'a str>()? {
            match key {
                "ts" | "TS" | "time" | "TIME" => {
                    ts = access.next_value()?;
                }
                "_SOURCE_REALTIME_TIMESTAMP" => {
                    rts = access.next_value()?;
                }
                "__REALTIME_TIMESTAMP" => {
                    rts = rts.or(Some(access.next_value()?));
                }
                "msg" | "MESSAGE" => {
                    text = access.next_value()?;
                }
                "level" | "LEVEL" => {
                    level = access.next_value()?;
                }
                "PRIORITY" => {
                    priority = access.next_value()?;
                }
                "logger" | "LOGGER" => {
                    logger = access.next_value()?;
                }
                "caller" | "CALLER" => {
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
        Ok(Message {
            ts: ts.or(rts),
            text,
            level,
            logger,
            caller,
            extra,
            extrax,
        })
    }
}

#[derive(Debug)]
pub struct FieldFilter {
    key: String,
    value: String,
    value_escaped: String,
    operator: Operator,
}

#[derive(Debug)]
enum Operator {
    Equal,
    NotEqual,
    Like,
    NotLike,
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
                Some(Self {
                    key: key.into(),
                    value: value.into(),
                    value_escaped: json::to_string(value).unwrap(),
                    operator: operator,
                })
            }
            _ => None,
        }
    }

    fn key_match(&self, key: &str) -> bool {
        if self.key.len() != key.len() {
            return false;
        }
        let norm = |b: u8| {
            if b == b'_' {
                b'-'
            } else {
                b.to_ascii_lowercase()
            }
        };
        self.key
            .as_bytes()
            .iter()
            .zip(key.as_bytes().iter())
            .position(|(&x, &y)| norm(x) != norm(y))
            .is_none()
    }

    fn value_match(&self, value: Option<&str>, escaped: bool) -> bool {
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
}

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
