// local imports
pub use self::{
    build::{Builder, Settings},
    filter::Filter,
};
use super::{ast, value::*};
use crate::{
    model::{Caller, Level},
    timestamp::Timestamp,
};
use once_cell::sync::Lazy;

// ---

pub mod build;
pub mod filter;

// ---

const MAX_PREDEFINED_FIELDS: usize = 8;

// ---

#[derive(Default)]
pub struct Record<'s> {
    pub ts: Option<Timestamp<'s>>,
    pub message: Option<ast::Scalar<'s>>,
    pub level: Option<Level>,
    pub logger: Option<&'s str>,
    pub caller: Option<Caller<'s>>,
    pub span: std::ops::Range<usize>,
    pub(crate) ast: ast::Container<'s>,
    pub(crate) predefined: heapless::Vec<ast::Index, MAX_PREDEFINED_FIELDS>,
}

impl<'s> Record<'s> {
    /// Returns an iterator over `Field` items for searching.
    ///
    /// The returned iterator borrows from `self` for the duration of the borrow.
    /// The `Field` items have a lifetime tied to the borrow of `self`,
    /// ensuring they do not outlive the `Record`.
    #[inline]
    pub fn fields_for_search<'r>(&'r self) -> Fields<'r, 's> {
        self.ast
            .roots()
            .into_iter()
            .next()
            .map(|root| Fields::new(root.children()))
            .unwrap_or_default()
    }

    #[inline]
    pub fn fields(&self) -> Fields<'_, 's> {
        // TODO: implement filtering out predefined fields
        self.fields_for_search()
    }

    #[inline]
    pub fn matches<F: Filter>(&self, filter: F) -> bool {
        filter.apply(self)
    }
}

impl<'s> From<Record<'s>> for ast::Container<'s> {
    #[inline]
    fn from(record: Record<'s>) -> Self {
        record.ast
    }
}

// ---

pub struct Fields<'r, 's> {
    inner: ast::Children<'r, 's>,
}

impl<'r, 's> Fields<'r, 's> {
    #[inline]
    pub fn new(inner: ast::Children<'r, 's>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn iter(&'r self) -> FieldsIter<'r, 's> {
        FieldsIter::new(self.inner.iter())
    }
}

impl<'r, 's> IntoIterator for Fields<'r, 's> {
    type Item = Field<'r, 's>;
    type IntoIter = FieldsIter<'r, 's>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        FieldsIter::new(self.inner.iter())
    }
}

impl Default for Fields<'_, '_> {
    #[inline]
    fn default() -> Self {
        static EMPTY: Lazy<ast::Container<'static>> = Lazy::new(|| {
            use ast::Build;
            let mut container = ast::Container::default();
            container
                .metaroot()
                .add_composite(ast::Composite::Object, |b| Ok(b))
                .unwrap();
            container
        });

        Self {
            inner: EMPTY.roots().iter().next().unwrap().children(),
        }
    }
}

// ---

pub struct FieldsIter<'r, 's> {
    inner: ast::SiblingsIter<'r, 's>,
}

impl<'r, 's> FieldsIter<'r, 's> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'r, 's>) -> Self {
        Self { inner }
    }
}

impl<'r, 's> Iterator for FieldsIter<'r, 's> {
    type Item = Field<'r, 's>;

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
