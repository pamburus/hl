use std::ops::Range;

use super::ast::Span;

pub trait Source: AsRef<[u8]> {
    type Slice: AsRef<[u8]> + ?Sized;

    fn slice(&self, span: Span) -> &Self::Slice;
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice;
}

impl Source for [u8] {
    type Slice = [u8];

    #[inline]
    fn slice(&self, span: Span) -> &Self::Slice {
        &self[Range::from(span)]
    }

    #[inline]
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice {
        self.get_unchecked(Range::from(span))
    }
}

impl Source for str {
    type Slice = str;

    #[inline]
    fn slice(&self, span: Span) -> &Self::Slice {
        &self[Range::from(span)]
    }

    #[inline]
    unsafe fn slice_unchecked(&self, span: Span) -> &Self::Slice {
        std::str::from_utf8_unchecked(self.as_bytes().get_unchecked(Range::from(span)))
    }
}
