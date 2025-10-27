use std::{
    ops::{Deref, Range},
    str::Utf8Error,
};

use super::ast::Span;

pub trait Source: Slice {
    type Ref<'a>: Source
    where
        Self: 'a;

    type Slice<'a>: Slice + ?Sized
    where
        Self: 'a;

    fn as_ref(&self) -> Self::Ref<'_>;
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
    fn slice(&self, span: Span) -> &Self::Slice<'_> {
        &self[Range::from(span)]
    }

    #[inline]
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice<'_> {
        unsafe { self.get_unchecked(Range::from(span)) }
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
    fn slice(&self, span: Span) -> &Self::Slice<'_> {
        &self[Range::from(span)]
    }

    #[inline]
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice<'_> {
        unsafe { std::str::from_utf8_unchecked(self.as_bytes().get_unchecked(Range::from(span))) }
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
    fn slice(&self, span: Span) -> &Self::Slice<'_> {
        self.deref().slice(span)
    }

    #[inline]
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice<'_> {
        unsafe { self.deref().slice_unchecked(span) }
    }
}

// ---

pub trait Slice {
    fn bytes(&self) -> &[u8];
    fn str(&self) -> Result<&str, Utf8Error>;
    unsafe fn str_unchecked(&self) -> &str;
}

impl Slice for [u8] {
    #[inline]
    fn bytes(&self) -> &[u8] {
        self
    }

    #[inline]
    fn str(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(self)
    }

    #[inline]
    unsafe fn str_unchecked(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self) }
    }
}

impl Slice for str {
    #[inline]
    fn bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    #[inline]
    fn str(&self) -> Result<&str, Utf8Error> {
        Ok(self)
    }

    #[inline]
    unsafe fn str_unchecked(&self) -> &str {
        self
    }
}

impl<T> Slice for T
where
    T: Deref,
    <T as Deref>::Target: Slice,
{
    #[inline]
    fn bytes(&self) -> &[u8] {
        self.deref().bytes()
    }

    #[inline]
    fn str(&self) -> Result<&str, Utf8Error> {
        self.deref().str()
    }

    #[inline]
    unsafe fn str_unchecked(&self) -> &str {
        unsafe { self.deref().str_unchecked() }
    }
}
