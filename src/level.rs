// std imports
use std::cmp::Ord;
use std::fmt;
use std::ops::Deref;
use std::result::Result;
use std::sync::Arc;

// third-party imports
use clap::{
    ValueEnum,
    builder::{EnumValueParser, TypedValueParser, ValueParserFactory},
};
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

// local imports
use crate::xerr::{HighlightQuoted, Suggestions};

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
#[strum(serialize_all = "lowercase")]
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

#[derive(Debug, Clone)]
pub struct ParseError {
    pub value: Arc<str>,
    pub suggestions: Suggestions,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid level {value}", value = self.value.hlq())
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
            .ok_or(ParseError {
                value: value.into(),
                suggestions: Suggestions::new(value, Level::iter().map(|level| level.as_ref().to_string())),
            })
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
mod tests;
