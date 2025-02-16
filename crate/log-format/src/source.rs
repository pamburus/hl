#[cfg(feature = "bytes")]
use std::{
    ops::{Deref, Range},
    sync::Arc,
};

#[cfg(not(feature = "bytes"))]
pub type Source = [u8];

#[cfg(feature = "bytes")]
pub type Source = Bytes;

#[cfg(feature = "bytes")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Bytes(Arc<[u8]>);

#[cfg(feature = "bytes")]
impl Bytes {
    #[inline]
    pub fn slice(&self, range: Range<usize>) -> ByteSlice {
        ByteSlice(self.0.clone(), range)
    }
}

#[cfg(feature = "bytes")]
impl From<Arc<[u8]>> for Bytes {
    #[inline]
    fn from(bytes: Arc<[u8]>) -> Self {
        Self(bytes)
    }
}

#[cfg(feature = "bytes")]
impl From<&'static [u8]> for Bytes {
    #[inline]
    fn from(bytes: &'static [u8]) -> Self {
        Self(Arc::from(bytes))
    }
}

#[cfg(feature = "bytes")]
impl<const N: usize> From<&'static [u8; N]> for Bytes {
    #[inline]
    fn from(bytes: &'static [u8; N]) -> Self {
        Self(Arc::from(&bytes[..]))
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
    type Slice<'a> = ByteSlice;

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
    fn slice(&self, range: Range<usize>) -> Option<Self::Slice<'_>> {
        if range.end <= self.0.len() {
            Some(self.slice(range))
        } else {
            None
        }
    }

    #[inline]
    unsafe fn slice_unchecked(&self, range: Range<usize>) -> Self::Slice<'_> {
        self.slice(range)
    }

    #[inline]
    fn is_boundary(&self, index: usize) -> bool {
        index <= self.0.len()
    }
}

#[cfg(feature = "bytes")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ByteSlice(Arc<[u8]>, Range<usize>);

#[cfg(feature = "bytes")]
impl ByteSlice {
    #[inline]
    pub fn slice(&self, mut range: Range<usize>) -> ByteSlice {
        range.start += self.1.start;
        range.end += self.1.start;
        ByteSlice(self.0.clone(), range)
    }
}

#[cfg(feature = "bytes")]
impl Deref for ByteSlice {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0[self.1.clone()]
    }
}

#[cfg(feature = "bytes")]
impl From<&str> for ByteSlice {
    #[inline]
    fn from(s: &str) -> Self {
        Self::from(s.as_bytes())
    }
}

#[cfg(feature = "bytes")]
impl From<&[u8]> for ByteSlice {
    #[inline]
    fn from(bytes: &[u8]) -> Self {
        Self(Arc::from(bytes), 0..bytes.len())
    }
}

#[cfg(feature = "bytes")]
impl<const N: usize> From<&[u8; N]> for ByteSlice {
    #[inline]
    fn from(bytes: &[u8; N]) -> Self {
        Self(Arc::from(&bytes[..]), 0..bytes.len())
    }
}

#[cfg(feature = "bytes")]
impl Default for ByteSlice {
    fn default() -> Self {
        Self::from(&[])
    }
}

#[cfg(feature = "bytes")]
impl PartialEq<[u8]> for ByteSlice {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.deref() == other
    }
}

#[cfg(feature = "bytes")]
impl<const N: usize> PartialEq<[u8; N]> for ByteSlice {
    #[inline]
    fn eq(&self, other: &[u8; N]) -> bool {
        self.deref() == other
    }
}
