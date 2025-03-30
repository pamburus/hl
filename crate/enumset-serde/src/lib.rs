// std imports
use std::fmt;

// third-party imports
use enumset::{EnumSet, EnumSetType};
use serde::de::{self, IntoDeserializer};

pub fn deserialize<'de, D, T: EnumSetType + de::Deserialize<'de>>(deserializer: D) -> Result<EnumSet<T>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(EnumSetDeserializer::default())
}

// ---

struct EnumSetDeserializer<T: EnumSetType>(EnumSet<T>);

impl<T: EnumSetType> Default for EnumSetDeserializer<T> {
    fn default() -> Self {
        Self(EnumSet::new())
    }
}

impl<'de, T: EnumSetType + de::Deserialize<'de>> de::Visitor<'de> for EnumSetDeserializer<T> {
    type Value = EnumSet<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a list of enum values or a comma-separated list of enum values")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut set = EnumSet::new();
        while let Some(value) = seq.next_element()? {
            set.insert(value);
        }
        Ok(set)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut set = EnumSet::new();
        for item in value.split(',') {
            let item = item.trim();
            let enum_value: T = T::deserialize(item.into_deserializer())?;
            set.insert(enum_value);
        }
        Ok(set)
    }
}
