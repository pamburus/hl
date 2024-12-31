use super::Record;
use crate::model::Level;

// ---

pub trait Filter {
    fn apply<'a>(&self, record: &Record<'a>) -> bool;

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
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl<T: Filter> Filter for &T {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl Filter for Level {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        record.level.map_or(false, |x| x <= *self)
    }
}

impl<T: Filter> Filter for Option<T> {
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

pub struct And<L: Filter, R: Filter> {
    lhs: L,
    rhs: R,
}

impl<L: Filter, R: Filter> Filter for And<L, R> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
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
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        self.lhs.apply(record) || self.rhs.apply(record)
    }
}

// ---

pub struct Pass;

impl Filter for Pass {
    #[inline]
    fn apply<'a>(&self, _: &Record<'a>) -> bool {
        true
    }
}
