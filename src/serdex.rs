use json::StreamDeserializer;
use std::ops::Range;

pub struct StreamDeserializerWithOffsets<'de, R, T>(pub StreamDeserializer<'de, R, T>);

impl<'de, R, T> Iterator for StreamDeserializerWithOffsets<'de, R, T>
where
    R: json::de::Read<'de>,
    T: serde::de::Deserialize<'de>,
{
    type Item = json::Result<(T, Range<usize>)>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let start_offset = self.0.byte_offset();
        self.0
            .next()
            .map(|res| res.map(|v| (v, start_offset..self.0.byte_offset())))
    }
}
