use super::Record;
use crate::model::Level;

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
