// third-party imports
use once_cell::sync::Lazy;

// local imports
pub use self::{
    build::{Builder, Settings},
    filter::Filter,
};
use super::{
    ast::{self, ObjectIter},
    value::*,
};
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

#[derive(Default)]
pub struct Record {
    ts: Option<TimestampSlot>,
    message: Option<ast::Index>,
    level: Option<Level>,
    logger: Option<ast::Index>,
    caller: Option<CallerSlot>,
    span: std::ops::Range<usize>,
    ast: ast::Segment,
    predefined: heapless::Vec<ast::Index, MAX_PREDEFINED_FIELDS>,
}

impl Record {
    pub fn ts(&self) -> Option<Timestamp> {
        match &self.ts {
            Some(ts) => Some(Timestamp::with_slot(self.ast.nodes(ts.index).value().text(), &ts.slot)),
            None => None,
        }
    }

    /// Returns an iterator over `Field` items for searching.
    ///
    /// The returned iterator borrows from `self` for the duration of the borrow.
    /// The `Field` items have a lifetime tied to the borrow of `self`,
    /// ensuring they do not outlive the `Record`.
    #[inline]
    pub fn fields_for_search(&self) -> SearchableFields {
        self.ast
            .roots()
            .into_iter()
            .next()
            .map(|root| Fields::new(root.children(), ()))
            .unwrap_or_default()
    }

    #[inline]
    pub fn fields(&self) -> VisibleFields {
        self.ast
            .roots()
            .into_iter()
            .next()
            .map(|root| {
                Fields::new(
                    root.children(),
                    PredefinedFieldFilter::new(self.predefined.iter().cloned()),
                )
            })
            .unwrap_or_default()
    }

    #[inline]
    pub fn matches<F: Filter>(&self, filter: F) -> bool {
        filter.apply(self)
    }
}

impl<'s> From<Record> for ast::Segment {
    #[inline]
    fn from(record: Record) -> Self {
        record.ast
    }
}

// ---

struct TimestampSlot {
    index: ast::Index,
    inner: crate::timestamp::Slot,
}

// ---

pub type SearchableFields<'r> = Fields<'r, ()>;
pub type VisibleFields<'r> = Fields<'r, PredefinedFieldFilter<std::iter::Cloned<core::slice::Iter<'r, ast::Index>>>>;

pub struct Fields<'r, HFF> {
    inner: ObjectIter<'r>,
    hff: HFF,
}

impl<'r, HFF> Fields<'r, HFF> {
    #[inline]
    pub fn new(inner: ObjectIter<'r>, hff: HFF) -> Self {
        Self { inner, hff }
    }
}

impl<'r, HFF> Fields<'r, HFF>
where
    HFF: HiddenFieldFilter,
{
    #[inline]
    pub fn iter(&'r self) -> FieldsIter<'r, HFF> {
        FieldsIter::new(self.inner.iter(), self.hff.clone())
    }
}

impl<'r, HFF> IntoIterator for Fields<'r, HFF>
where
    HFF: HiddenFieldFilter,
{
    type Item = Field<'r>;
    type IntoIter = FieldsIter<'r, HFF>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        FieldsIter::new(self.inner.into_iter(), self.hff)
    }
}

impl<'r, 's> IntoIterator for Fields<'r, 's, ()> {
    type Item = Field<'r, 's>;
    type IntoIter = FieldsIter<'r, 's, ()>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        FieldsIter::new(self.inner.into_iter(), ())
    }
}

impl<HFF: Default> Default for Fields<'_, '_, HFF> {
    #[inline]
    fn default() -> Self {
        static EMPTY: Lazy<ast::Container<'static>> = Lazy::new(|| {
            use ast::Build;
            let mut container = ast::Container::default();
            container
                .metaroot()
                .add_composite(ast::Composite::Object, |b| (b, Ok(())))
                .1
                .unwrap();
            container
        });

        Self {
            inner: EMPTY.roots().iter().next().unwrap().children(),
            hff: Default::default(),
        }
    }
}

// ---

pub struct FieldsIter<'r, HFF> {
    inner: ObjectIter<'r>,
    hff: HFF,
}

impl<'r, 's, HFF> FieldsIter<'r, 's, HFF> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'r, 's>, hff: HFF) -> Self {
        Self { inner, hff }
    }
}

impl<'r, 's> Iterator for FieldsIter<'r, 's, ()> {
    type Item = Field<'r, 's>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Field::from_node)
    }
}

impl<'r, 's, HFF> Iterator for FieldsIter<'r, 's, HFF>
where
    HFF: HiddenFieldFilter,
{
    type Item = Field<'r, 's>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.inner.next() {
            if !self.hff.is_hidden(node.index()) {
                return Some(Field::from_node(node));
            }
        }
        None
    }
}

// ---

pub trait HiddenFieldFilter: Clone {
    fn is_hidden(&mut self, index: ast::Index) -> bool;
}

#[derive(Clone, Default)]
pub struct PredefinedFieldFilter<I> {
    head: Option<ast::Index>,
    tail: I,
}

impl<I> PredefinedFieldFilter<I>
where
    I: Iterator<Item = ast::Index>,
{
    #[inline]
    fn new(mut tail: I) -> Self {
        let head = tail.next();
        Self { head, tail }
    }
}

impl<I> HiddenFieldFilter for PredefinedFieldFilter<I>
where
    I: Iterator<Item = ast::Index> + Clone,
{
    #[inline]
    fn is_hidden(&mut self, index: ast::Index) -> bool {
        let Some(head) = self.head else {
            return false;
        };
        if head != index {
            return false;
        }

        self.head = self.tail.next();
        return true;
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
