#[cfg(not(feature = "bytes"))]
pub type Source = [u8];

#[cfg(feature = "bytes")]
pub type Source = Bytes;

#[cfg(feature = "bytes")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Bytes(bytes::Bytes);

#[cfg(feature = "bytes")]
impl From<bytes::Bytes> for Bytes {
    #[inline]
    fn from(bytes: bytes::Bytes) -> Self {
        Self(bytes)
    }
}

#[cfg(feature = "bytes")]
impl From<&'static [u8]> for Bytes {
    #[inline]
    fn from(bytes: &'static [u8]) -> Self {
        Self(bytes::Bytes::from_static(bytes))
    }
}

#[cfg(feature = "bytes")]
impl<const N: usize> From<&'static [u8; N]> for Bytes {
    #[inline]
    fn from(bytes: &'static [u8; N]) -> Self {
        Self(bytes::Bytes::from_static(bytes))
    }
}

#[cfg(feature = "bytes")]
impl AsRef<[u8]> for Bytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[cfg(all(feature = "bytes", feature = "logos"))]
impl logos::Source for Bytes {
    type Slice<'a> = bytes::Bytes;

    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn read<'a, Chunk>(&'a self, offset: usize) -> Option<Chunk>
    where
        Chunk: logos::source::Chunk<'a>,
    {
        if offset + (Chunk::SIZE - 1) < self.len() {
            Some(unsafe { Chunk::from_ptr(self.0.as_ptr().add(offset)) })
        } else {
            None
        }
    }

    #[inline]
    unsafe fn read_byte_unchecked(&self, offset: usize) -> u8 {
        *self.0.get_unchecked(offset)
    }

    #[inline]
    fn slice(&self, range: std::ops::Range<usize>) -> Option<Self::Slice<'_>> {
        if range.end <= self.0.len() {
            Some(self.0.slice(range))
        } else {
            None
        }
    }

    #[inline]
    unsafe fn slice_unchecked(&self, range: std::ops::Range<usize>) -> Self::Slice<'_> {
        self.0.slice(range)
    }

    #[inline]
    fn is_boundary(&self, index: usize) -> bool {
        index <= self.0.len()
    }
}
