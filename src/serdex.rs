use json::StreamDeserializer;
use std::ops::Range;

pub struct StreamDeserializerWithOffsets<'de, R, T> {
    pub inner: StreamDeserializer<'de, R, T>,
    pub source: &'de [u8],
}

impl<'de, R, T> Iterator for StreamDeserializerWithOffsets<'de, R, T>
where
    R: json::de::Read<'de>,
    T: serde::de::Deserialize<'de>,
{
    type Item = json::Result<(T, Range<usize>)>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let mut start_offset = self.inner.byte_offset();
        if let Some(i) = self.source[start_offset..]
            .iter()
            .position(|&b| !b.is_ascii_whitespace())
        {
            start_offset += i;
        }

        self.inner
            .next()
            .map(|res| res.map(|v| (v, start_offset..self.inner.byte_offset())))
    }
}
