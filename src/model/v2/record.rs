// local imports
pub use self::filter::Filter;
use super::{ast, value::*};
use crate::{
    model::{Caller, Level},
    timestamp::Timestamp,
};

// ---

pub mod build;
pub mod filter;

// ---

const MAX_PREDEFINED_FIELDS: usize = 8;

// ---

pub struct Record<'s> {
    pub ts: Option<Timestamp<'s>>,
    pub message: Option<Value<'s>>,
    pub level: Option<Level>,
    pub logger: Option<&'s str>,
    pub caller: Option<Caller<'s>>,
    pub fields: Fields<'s>,
    predefined: heapless::Vec<Field<'s>, MAX_PREDEFINED_FIELDS>,
}

impl<'s> Record<'s> {
    #[inline]
    pub fn new(fields: Fields<'s>) -> Self {
        Self {
            ts: None,
            message: None,
            level: None,
            logger: None,
            caller: None,
            fields,
            predefined: heapless::Vec::new(),
        }
    }

    #[inline]
    pub fn fields_for_search<'r>(&'r self) -> impl Iterator<Item = Field<'s>> + 'r {
        self.fields.iter().chain(self.predefined.iter().copied())
    }

    #[inline]
    pub fn matches<F: Filter>(&self, filter: F) -> bool {
        filter.apply(self)
    }
}

// ---

#[derive(Default, Debug)]
pub struct RecordStem<'s> {
    pub ts: Option<Timestamp<'s>>,
    pub message: Option<Value<'s>>,
    pub level: Option<Level>,
    pub logger: Option<&'s str>,
    pub caller: Option<Caller<'s>>,
    predefined: heapless::Vec<Field<'s>, MAX_PREDEFINED_FIELDS>,
}

impl<'s> RecordStem<'s> {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn into_record(self, fields: Fields<'s>) -> Record<'s> {
        Record {
            ts: self.ts,
            message: self.message,
            level: self.level,
            logger: self.logger,
            caller: self.caller,
            fields,
            predefined: self.predefined,
        }
    }
}

// ---

pub struct Fields<'s> {
    inner: ast::Children<'s>,
}

impl<'s> Fields<'s> {
    #[inline]
    fn new(inner: ast::Children<'s>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn iter(&self) -> FieldsIter<'s> {
        FieldsIter::new(self.inner.iter())
    }
}

// ---

struct FieldsIter<'s> {
    inner: ast::SiblingsIter<'s>,
}

impl<'s> FieldsIter<'s> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> Iterator for FieldsIter<'s> {
    type Item = Field<'s>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Field::from_node)
    }
}

// ---

pub trait RecordWithSourceConstructor<'r, 's> {
    fn with_source(&'r self, source: &'s [u8]) -> RecordWithSource<'r, 's>;
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
