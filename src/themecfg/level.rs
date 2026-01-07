use derive_more::{Deref, DerefMut, From, Into, PartialEq};
use serde::{Deserialize, Serialize};

use crate::level::Level as InnerLevel;

#[derive(Clone, Debug, Deref, DerefMut, PartialEq, Eq, Hash, Ord, PartialOrd, From, Into)]
pub struct Level {
    inner: Option<InnerLevel>,
}

impl From<InnerLevel> for Level {
    fn from(level: InnerLevel) -> Self {
        Level { inner: Some(level) }
    }
}

impl Serialize for Level {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.inner {
            Some(level) => level.serialize(serializer),
            None => serializer.serialize_str("unknown"),
        }
    }
}

impl<'de> Deserialize<'de> for Level {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "unknown" {
            Ok(Level { inner: None })
        } else {
            InnerLevel::deserialize(serde::de::value::StringDeserializer::<D::Error>::new(s))
                .map(|level| Level { inner: Some(level) })
                .map_err(serde::de::Error::custom)
        }
    }
}

#[cfg(test)]
mod tests;
