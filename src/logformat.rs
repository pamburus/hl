use std::marker::PhantomData;
use std::result::Result;

use serde_json as json;

use crate::model::RawRecord;

pub trait RawValue {
    fn get(&self) -> &str;
}

pub trait Format {
    type RawValue: ?Sized;
    type Error;
    type Stream<'a>: Iterator<Item = Result<RawRecord<'a, Self::RawValue>, Self::Error>>
    where
        <Self as Format>::RawValue: 'a;

    fn stream<'a>(data: &'a [u8]) -> Self::Stream<'a>;
}

pub trait JsonSettings {
    type Read<'de>: json::de::Read<'de>;
}

pub struct FormatJson<S: JsonSettings> {
    marker: PhantomData<fn() -> S>,
}

impl<S: JsonSettings> Format for FormatJson<S> {
    type RawValue = json::value::RawValue;
    type Error = json::Error;
    type Stream<'de> =
        json::de::StreamDeserializer<'de, S::Read<'de>, RawRecord<'de, Self::RawValue>>;

    fn stream<'de>(data: &'de [u8]) -> Self::Stream<'de> {
        json::Deserializer::from_slice(data).into_iter::<RawRecord<'de, Self::RawValue>>()
    }
}

impl RawValue for json::value::RawValue {
    fn get(&self) -> &str {
        self.get()
    }
}
