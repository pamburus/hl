// std imports
use std::{fmt, ops::Deref, str::FromStr};

// third-party imports
use enumset::EnumSetType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumSet<T: EnumSetType>(enumset::EnumSet<T>);

#[cfg(feature = "serde")]
impl<'de, T: EnumSetType + serde::de::Deserialize<'de>> serde::de::Deserialize<'de> for EnumSet<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        Ok(Self(enumset_serde::deserialize(deserializer)?))
    }
}

impl<T: EnumSetType> EnumSet<T> {
    pub const fn all() -> Self {
        Self(enumset::EnumSet::all())
    }

    pub const fn empty() -> Self {
        Self(enumset::EnumSet::empty())
    }
}

impl<T: EnumSetType> Deref for EnumSet<T> {
    type Target = enumset::EnumSet<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: EnumSetType> From<enumset::EnumSet<T>> for EnumSet<T> {
    fn from(set: enumset::EnumSet<T>) -> Self {
        Self(set)
    }
}

impl<T: EnumSetType + fmt::Display> fmt::Display for EnumSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for item in self.0.iter() {
            if first {
                first = false;
            } else {
                write!(f, ",")?;
            }
            write!(f, "{}", item)?;
        }
        Ok(())
    }
}

impl<T: EnumSetType + FromStr> FromStr for EnumSet<T> {
    type Err = <T as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut set = enumset::EnumSet::new();
        for item in s.split(',') {
            let item = item.trim();
            let enum_value: T = T::from_str(item)?;
            set.insert(enum_value);
        }
        Ok(EnumSet(set))
    }
}
