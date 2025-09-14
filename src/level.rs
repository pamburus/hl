// std imports
use std::cmp::Ord;
use std::fmt;
use std::ops::Deref;
use std::result::Result;

// third-party imports
use clap::{
    ValueEnum,
    builder::{EnumValueParser, TypedValueParser, ValueParserFactory},
};
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter};

// ---

#[derive(
    ValueEnum,
    Clone,
    Copy,
    Debug,
    Deserialize,
    Serialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Enum,
    EnumIter,
    AsRefStr,
)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

// ---

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
pub enum InfallibleLevel {
    Valid(Level),
    Invalid(String),
}

impl InfallibleLevel {
    pub const fn new(level: Level) -> Self {
        Self::Valid(level)
    }
}

impl From<Level> for InfallibleLevel {
    fn from(value: Level) -> Self {
        InfallibleLevel::Valid(value)
    }
}

// ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to parse level")
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

// ---

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, Ord, PartialEq, PartialOrd, Enum)]
pub struct RelaxedLevel(Level);

impl From<RelaxedLevel> for Level {
    fn from(relaxed: RelaxedLevel) -> Level {
        relaxed.0
    }
}

impl Deref for RelaxedLevel {
    type Target = Level;

    fn deref(&self) -> &Level {
        &self.0
    }
}

impl ValueParserFactory for RelaxedLevel {
    type Parser = LevelValueParser;
    fn value_parser() -> Self::Parser {
        LevelValueParser
    }
}

impl TryFrom<&str> for RelaxedLevel {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        LevelValueParser::alternate_values()
            .iter()
            .find(|(_, values)| values.iter().cloned().any(|x| value.eq_ignore_ascii_case(x)))
            .map(|(level, _)| RelaxedLevel(*level))
            .ok_or(ParseError)
    }
}

// ---

#[derive(Clone, Debug)]
pub struct LevelValueParser;

impl TypedValueParser for LevelValueParser {
    type Value = RelaxedLevel;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<RelaxedLevel, clap::Error> {
        for (level, values) in Self::alternate_values() {
            if values.iter().cloned().any(|x| value.eq_ignore_ascii_case(x)) {
                return Ok(RelaxedLevel(*level));
            }
        }

        let inner = EnumValueParser::<Level>::new();
        let val = inner.parse_ref(cmd, arg, value)?;
        Ok(RelaxedLevel(val))
    }
}

impl LevelValueParser {
    fn alternate_values<'a>() -> &'a [(Level, &'a [&'a str])] {
        &[
            (Level::Error, &["error", "err", "e"]),
            (Level::Warning, &["warning", "warn", "wrn", "w"]),
            (Level::Info, &["info", "inf", "i"]),
            (Level::Debug, &["debug", "dbg", "d"]),
            (Level::Trace, &["trace", "trc", "t"]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relaxed_level_from_conversion() {
        let relaxed = RelaxedLevel(Level::Info);
        let level: Level = relaxed.into();
        assert_eq!(level, Level::Info);

        let relaxed = RelaxedLevel(Level::Error);
        let level: Level = Level::from(relaxed);
        assert_eq!(level, Level::Error);
    }

    #[test]
    fn test_relaxed_level_deref() {
        let relaxed = RelaxedLevel(Level::Warning);
        assert_eq!(*relaxed, Level::Warning);
        assert_eq!(relaxed.deref(), &Level::Warning);
    }

    #[test]
    fn test_relaxed_level_try_from_str() {
        // Test case-insensitive parsing
        assert_eq!(RelaxedLevel::try_from("info").unwrap().0, Level::Info);
        assert_eq!(RelaxedLevel::try_from("INFO").unwrap().0, Level::Info);
        assert_eq!(RelaxedLevel::try_from("Info").unwrap().0, Level::Info);

        assert_eq!(RelaxedLevel::try_from("error").unwrap().0, Level::Error);
        assert_eq!(RelaxedLevel::try_from("ERROR").unwrap().0, Level::Error);

        assert_eq!(RelaxedLevel::try_from("warn").unwrap().0, Level::Warning);
        assert_eq!(RelaxedLevel::try_from("warning").unwrap().0, Level::Warning);

        // Test invalid input
        assert!(RelaxedLevel::try_from("invalid").is_err());
    }

    #[test]
    fn test_level_value_parser() {
        let _parser = LevelValueParser;

        // Test that alternate values are available
        let alternate_values = LevelValueParser::alternate_values();
        assert!(!alternate_values.is_empty());

        // Verify some expected alternate values exist
        let has_warning = alternate_values
            .iter()
            .any(|(level, values)| *level == Level::Warning && values.contains(&"warning"));
        assert!(has_warning, "Should have 'warning' as alternate for Warn level");
    }
}
