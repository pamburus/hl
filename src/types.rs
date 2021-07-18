// std imports
use std::cmp::Ord;
use std::str::FromStr;

// third-party imports
use enum_map::Enum;
use serde::Deserialize;

// local imports
use crate::error::{Error, Result};

// ---

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Enum)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    Error,
    Warning,
    Info,
    Debug,
}

impl FromStr for Level {
    type Err = Error;

    fn from_str(s: &str) -> Result<Level> {
        let matches = |value| s.eq_ignore_ascii_case(value);
        if matches("e") || matches("error") {
            Ok(Level::Error)
        } else if matches("w") || matches("warn") || matches("warning") {
            Ok(Level::Warning)
        } else if matches("i") || matches("info") {
            Ok(Level::Info)
        } else if matches("d") || matches("debug") {
            Ok(Level::Debug)
        } else {
            Err(Error::InvalidLevel {
                value: s.into(),
                valid_values: vec![
                    "error".into(),
                    "warning".into(),
                    "info".into(),
                    "debug".into(),
                ],
            })
        }
    }
}

// ---

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
pub enum FieldKind {
    Time,
    Level,
    Logger,
    Message,
    Caller,
}
