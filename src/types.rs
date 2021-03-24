use std::cmp::Ord;
use std::str::FromStr;

use crate::error::{Error, Result};

#[derive(Ord, PartialOrd, PartialEq, Eq, Debug, Hash, Clone)]
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
                valid_values: vec!["e".into(), "w".into(), "i".into(), "d".into()],
            })
        }
    }
}
