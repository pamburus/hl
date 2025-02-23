// std imports
use std::{cmp::Ordering, collections::HashSet, str::FromStr, sync::Arc};

// third-party imports
use chrono::{DateTime, Utc};
use regex::Regex;
use wildflower::Pattern;

// workspace imports
use encstr::{AnyEncodedString, EncodedString};

// local imports
use super::{Record, Value};
use crate::{
    error::{Error, Result},
    model::{Caller, Level},
    types::FieldKind,
};

// ---

pub trait Filter {
    fn apply<'a>(&self, record: &Record) -> bool;

    #[inline]
    fn and<F>(self, rhs: F) -> And<Self, F>
    where
        Self: Sized,
        F: Filter,
    {
        And { lhs: self, rhs }
    }

    #[inline]
    fn or<F>(self, rhs: F) -> Or<Self, F>
    where
        Self: Sized,
        F: Filter,
    {
        Or { lhs: self, rhs }
    }
}

impl<T: Filter + ?Sized> Filter for Box<T> {
    #[inline]
    fn apply<'a>(&self, record: &Record) -> bool {
        (**self).apply(record)
    }
}

impl<T: Filter + ?Sized> Filter for Arc<T> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl<T: Filter> Filter for &T {
    #[inline]
    fn apply<'a>(&self, record: &Record) -> bool {
        (**self).apply(record)
    }
}

impl Filter for Level {
    #[inline]
    fn apply<'a>(&self, record: &Record) -> bool {
        record.level.map_or(false, |x| x <= *self)
    }
}

impl<T: Filter> Filter for Option<T> {
    #[inline]
    fn apply<'a>(&self, record: &Record) -> bool {
        if let Some(filter) = self {
            filter.apply(record)
        } else {
            true
        }
    }
}

// ---

pub struct And<L: Filter, R: Filter> {
    lhs: L,
    rhs: R,
}

impl<L: Filter, R: Filter> Filter for And<L, R> {
    #[inline]
    fn apply<'a>(&self, record: &Record) -> bool {
        self.lhs.apply(record) && self.rhs.apply(record)
    }
}

// ---

pub struct Or<L: Filter, R: Filter> {
    lhs: L,
    rhs: R,
}

impl<L: Filter, R: Filter> Filter for Or<L, R> {
    #[inline]
    fn apply<'a>(&self, record: &Record) -> bool {
        self.lhs.apply(record) || self.rhs.apply(record)
    }
}

// ---

pub struct Pass;

impl Filter for Pass {
    #[inline]
    fn apply<'a>(&self, _: &Record) -> bool {
        true
    }
}

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

    fn match_value(&self, value: Option<EncodedString>) -> bool {
        let apply = |value| self.op.apply(self.match_policy.matches(value));
        if let Some(value) = value {
            match value {
                EncodedString::Raw(value) => apply(&value),
                EncodedString::Json(value) => {
                    let mut buf = Vec::new();
                    if value.decode(&mut buf).is_ok() {
                        if let Ok(s) = std::str::from_utf8(&buf) {
                            apply(s)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    }

    fn match_value_partial(&self, subkey: KeyMatcher, value: Value) -> bool {
        if let Value::Object(value) = value {
            for field in value.iter() {
                match subkey.match_key(field.key) {
                    None => {
                        continue;
                    }
                    Some(KeyMatch::Full) => {
                        return self.match_value(field.value.as_text());
                    }
                    Some(KeyMatch::Partial(subkey)) => {
                        return self.match_value_partial(subkey, field.value);
                    }
                }
            }
        }
        false
    }
}

impl Filter for FieldFilter {
    fn apply<'a>(&self, record: &Record) -> bool {
        match &self.key {
            FieldFilterKey::Predefined(kind) => match kind {
                FieldKind::Time => {
                    if let Some(ts) = &record.ts {
                        self.match_value(Some(EncodedString::raw(ts.raw())))
                    } else {
                        false
                    }
                }
                FieldKind::Message => {
                    if let Some(message) = record.message {
                        self.match_value(Some(message.as_text()))
                    } else {
                        false
                    }
                }
                FieldKind::Logger => {
                    if let Some(logger) = record.logger {
                        self.match_value(Some(EncodedString::raw(logger)))
                    } else {
                        false
                    }
                }
                FieldKind::Caller => {
                    if let Some(Caller::Text(caller)) = record.caller {
                        self.match_value(Some(EncodedString::raw(caller)))
                    } else {
                        false
                    }
                }
                _ => true,
            },
            FieldFilterKey::Custom(_) => {
                for field in record.fields_for_search() {
                    match self.match_custom_key(field.key) {
                        None => {}
                        Some(KeyMatch::Full) => {
                            if self.match_value(field.value.as_text()) {
                                return true;
                            }
                        }
                        Some(KeyMatch::Partial(subkey)) => {
                            if self.match_value_partial(subkey, field.value) {
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

impl Filter for FieldFilterSet {
    #[inline]
    fn apply<'a>(&self, record: &Record) -> bool {
        self.0.iter().all(|field| field.apply(record))
    }
}

// ---

#[derive(Default)]
pub struct CombinedFilter {
    pub fields: FieldFilterSet,
    pub level: Option<Level>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

impl CombinedFilter {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.fields.0.is_empty() && self.level.is_none() && self.since.is_none() && self.until.is_none()
    }
}

impl Filter for CombinedFilter {
    fn apply<'a>(&self, record: &Record) -> bool {
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
