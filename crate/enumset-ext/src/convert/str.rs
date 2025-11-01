// std imports
use std::{fmt, ops::Deref, str::FromStr};

// third-party imports
use enumset::EnumSetType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumSet<T: EnumSetType>(enumset::EnumSet<T>);

impl<T> Default for EnumSet<T>
where
    T: EnumSetType,
{
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: EnumSetType> EnumSet<T> {
    pub const fn all() -> Self {
        Self(enumset::EnumSet::all())
    }

    pub const fn empty() -> Self {
        Self(enumset::EnumSet::empty())
    }

    #[cfg(feature = "clap")]
    pub const fn clap_parser() -> ClapParser<T> {
        ClapParser::new()
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

#[cfg(feature = "serde")]
impl<'de, T: EnumSetType + serde::de::Deserialize<'de>> serde::de::Deserialize<'de> for EnumSet<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        Ok(Self(enumset_serde::deserialize(deserializer)?))
    }
}

#[cfg(feature = "clap")]
#[derive(Debug, Clone)]
pub struct ClapParser<T>(std::marker::PhantomData<fn(T) -> T>);

#[cfg(feature = "clap")]
impl<T> ClapParser<T> {
    pub const fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

#[cfg(feature = "clap")]
impl<T> Default for ClapParser<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "clap")]
impl<T> clap::builder::TypedValueParser for ClapParser<T>
where
    T: EnumSetType + fmt::Display + Sync + Send + FromStr + 'static,
    enumset::EnumSet<T>: Send + Sync,
{
    type Value = EnumSet<T>;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        clap::builder::StringValueParser::new()
            .parse_ref(cmd, arg, value)
            .and_then(|s| {
                let mut set = enumset::EnumSet::new();
                for value in s.split(',') {
                    let value = value.trim();
                    let value: T = T::from_str(value).map_err(|_| {
                        clap::builder::PossibleValuesParser::new(self.possible_values().unwrap())
                            .parse_ref(cmd, arg, std::ffi::OsStr::new(value))
                            .unwrap_err()
                    })?;
                    set.insert(value);
                }
                Ok(EnumSet(set))
            })
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = clap::builder::PossibleValue> + '_>> {
        Some(Box::new(
            EnumSet::all()
                .iter()
                .map(|v: T| clap::builder::PossibleValue::new(v.to_string())),
        ))
    }
}

#[cfg(test)]
mod tests;
