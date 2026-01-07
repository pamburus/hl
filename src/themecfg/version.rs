// std imports
use std::{
    fmt,
    hash::Hash,
    str::{self, FromStr},
};

// third-party imports
use serde::{Deserialize, Deserializer, Serialize, de::Visitor};
use static_assertions::const_assert;

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

    /// Version 1.1 (added unknown style and level)
    pub const V1_1: Self = Self::new(1, 1);

    /// Current supported version
    pub const CURRENT: Self = Self::V1_1;

    /// Check if this version is compatible with a supported version
    pub fn is_compatible_with(&self, supported: &Version) -> bool {
        // Same major version and minor <= supported
        self.major == supported.major && self.minor <= supported.minor
    }

    /// Const fn to parse a version string at compile time
    pub const fn must_parse(s: &str) -> Version {
        match Self::parse(s) {
            Some(v) => v,
            None => panic!("invalid version string"),
        }
    }

    /// Const fn to parse a version string at compile time
    pub const fn parse(s: &str) -> Option<Version> {
        let bytes = s.as_bytes();
        let len = bytes.len();

        if len == 0 {
            return None;
        }

        // Find the dot position
        let mut dot_pos = None;
        let mut i = 0;
        while i < len {
            if bytes[i] == b'.' {
                if dot_pos.is_some() {
                    // Multiple dots found
                    return None;
                }
                dot_pos = Some(i);
            }
            i += 1;
        }

        // Dot is mandatory
        let dot_pos = match dot_pos {
            Some(pos) => pos,
            None => return None,
        };

        // Helper function to parse a number segment
        const fn parse_segment(bytes: &[u8], start: usize, end: usize) -> Option<u32> {
            if start >= end {
                return None;
            }

            let mut result = 0u32;
            let mut i = start;
            while i < end {
                let c = bytes[i];
                if c < b'0' || c > b'9' {
                    return None;
                }
                // Check for leading zero
                if i == start && c == b'0' && end - start > 1 {
                    return None;
                }
                result = result * 10 + (c - b'0') as u32;
                i += 1;
            }
            Some(result)
        }

        // Parse major and minor
        let major = match parse_segment(bytes, 0, dot_pos) {
            Some(v) => v,
            None => return None,
        };

        let minor = match parse_segment(bytes, dot_pos + 1, len) {
            Some(v) => v,
            None => return None,
        };

        Some(Self::new(major, minor))
    }

    const fn equals(&self, other: &Version) -> bool {
        self.major == other.major && self.minor == other.minor
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
        Self::parse(s).ok_or_else(|| Error::InvalidVersion(s.into()))
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

// Build-time check to ensure CURRENT version matches the schema version
const CURRENT_VERSION: Version = Version::must_parse(env!("HL_BUILD_THEME_VERSION"));

// Ensure that the CURRENT version matches the parsed schema version at compile time
const_assert!(Version::CURRENT.equals(&CURRENT_VERSION));

#[cfg(test)]
pub(crate) mod tests;
