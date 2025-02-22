use std::ops::{Deref, Range};

use super::ast::Span;

pub trait Source {
    type Slice<'a>: AsRef<[u8]> + ?Sized
    where
        Self: 'a;

    fn bytes(&self) -> &[u8];
    fn slice(&self, span: Span) -> &Self::Slice<'_>;
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice<'_>;
}

impl Source for [u8] {
    type Slice<'a> = [u8];

    #[inline]
    fn bytes(&self) -> &[u8] {
        self
    }

    #[inline]
    fn slice(&self, span: Span) -> &Self::Slice<'_> {
        &self[Range::from(span)]
    }

    #[inline]
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice<'_> {
        self.get_unchecked(Range::from(span))
    }
}

impl Source for str {
    type Slice<'a> = str;

    #[inline]
    fn bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    #[inline]
    fn slice(&self, span: Span) -> &Self::Slice<'_> {
        &self[Range::from(span)]
    }

    #[inline]
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice<'_> {
        std::str::from_utf8_unchecked(self.as_bytes().get_unchecked(Range::from(span)))
    }
}

impl<T> Source for T
where
    T: Deref,
    <T as Deref>::Target: Source,
{
    type Slice<'a>
        = <T::Target as Source>::Slice<'a>
    where
        T: 'a;

    #[inline]
    fn bytes(&self) -> &[u8] {
        self.deref().bytes()
    }

    #[inline]
    fn slice(&self, span: Span) -> &Self::Slice<'_> {
        self.deref().slice(span)
    }

    #[inline]
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice<'_> {
        self.deref().slice_unchecked(span)
    }
}
