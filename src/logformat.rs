use std::result::Result;

use serde_json as json;

use crate::model::RawRecord;

pub trait RawValueLike {
    fn get(&self) -> &str;
}

pub trait Format {
    type RawValue: RawValueLike + ?Sized;
    type Error;
    type Stream<'a>: Iterator<Item = Result<RawRecord<'a, Self::RawValue>, Self::Error>>
    where
        <Self as Format>::RawValue: 'a;

    fn stream<'a>(data: &'a [u8]) -> Self::Stream<'a>;
}

pub struct FormatJson {}

impl Format for FormatJson {
    type RawValue = json::value::RawValue;
    type Error = json::Error;
    type Stream<'de> =
        json::de::StreamDeserializer<'de, json::de::SliceRead<'de>, RawRecord<'de, Self::RawValue>>;

    fn stream<'de>(data: &'de [u8]) -> Self::Stream<'de> {
        json::Deserializer::from_slice(data).into_iter::<RawRecord<'de, Self::RawValue>>()
    }
}

impl RawValueLike for json::value::RawValue {
    fn get(&self) -> &str {
        self.get()
    }
}
