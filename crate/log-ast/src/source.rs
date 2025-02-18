use std::ops::{Deref, Range};

use super::ast::Span;

pub trait Source {
    type Ref<'a>: Source
    where
        Self: 'a;

    type Slice<'a>: AsRef<[u8]> + ?Sized
    where
        Self: 'a;

    fn as_ref(&self) -> Self::Ref<'_>;
    fn bytes(&self) -> &[u8];
    fn slice(&self, span: Span) -> &Self::Slice<'_>;
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice<'_>;
}

impl Source for [u8] {
    type Ref<'a> = &'a [u8];
    type Slice<'a> = [u8];

    #[inline]
    fn as_ref(&self) -> Self::Ref<'_> {
        self
    }

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
    type Ref<'a> = &'a str;
    type Slice<'a> = str;

    #[inline]
    fn as_ref(&self) -> Self::Ref<'_> {
        self
    }

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
    type Ref<'a>
        = <T::Target as Source>::Ref<'a>
    where
        T: 'a;

    type Slice<'a>
        = <T::Target as Source>::Slice<'a>
    where
        T: 'a;

    #[inline]
    fn as_ref(&self) -> Self::Ref<'_> {
        self.deref().as_ref()
    }

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
