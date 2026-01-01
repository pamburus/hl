// std imports
use std::{
    fmt,
    hash::Hash,
    str::{self, FromStr},
};

// third-party imports
use serde::{Deserialize, Deserializer, Serialize, de::Visitor};

use crate::themecfg::MergeOptions;

// relative imports
use super::{Error, MergeFlag, MergeFlags, Result};

/// Theme version with major.minor components
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

impl Version {
    /// Create a new theme version
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Version 0 (the latest v0 version)
    pub const V0: Self = Self::V0_0;

    /// Version 0.0 (implicit, no version field in theme)
    pub const V0_0: Self = Self::new(0, 0);

    /// Version 1 (the latest v1 version)
    pub const V1: Self = Self::V1_0;

    /// Version 1.0 (first versioned theme format)
    pub const V1_0: Self = Self::new(1, 0);

    /// Current supported version
    pub const CURRENT: Self = Self::V1;

    /// Check if this version is compatible with a supported version
    pub fn is_compatible_with(&self, supported: &Version) -> bool {
        // Same major version and minor <= supported
        self.major == supported.major && self.minor <= supported.minor
    }
}

impl MergeOptions for Version {
    type Output = MergeFlags;

    fn merge_options(&self) -> Self::Output {
        match self {
            Self { major: 0, .. } => {
                MergeFlag::ReplaceElements | MergeFlag::ReplaceHierarchies | MergeFlag::ReplaceModes
            }
            Self { major: 1, .. } => MergeFlags::new(),
            _ => MergeFlags::new(),
        }
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        let err = || Error::InvalidVersion(s.into());

        if parts.len() != 2 {
            return Err(err());
        }

        let major: u32 = parts[0].parse().map_err(|_| err())?;
        let minor: u32 = parts[1].parse().map_err(|_| err())?;

        // Reject leading zeros (except "0" itself)
        if (parts[0].len() > 1 && parts[0].starts_with('0')) || (parts[1].len() > 1 && parts[1].starts_with('0')) {
            return Err(err());
        }

        Ok(Version { major, minor })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ThemeVersionVisitor;

        impl<'de> Visitor<'de> for ThemeVersionVisitor {
            type Value = Version;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a version string like \"1.0\"")
            }

            fn visit_str<E>(self, value: &str) -> Result<Version, E>
            where
                E: serde::de::Error,
            {
                Version::from_str(value).map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_str(ThemeVersionVisitor)
    }
}

#[cfg(test)]
pub(crate) mod tests;
