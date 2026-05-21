// std imports
use std::cmp::Ord;

// third-party imports
use serde::{Deserialize, Serialize};

// ---

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FieldKind {
    Time,
    Level,
    Logger,
    Message,
    Caller,
    CallerFile,
    CallerLine,
}

// ---

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum InputFormat {
    Json,
    Logfmt,
}

// ---

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum UnixTimestampUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl UnixTimestampUnit {
    pub fn guess(ts: i64) -> Self {
        match ts {
            Self::TS_UNIX_AUTO_S_MIN..=Self::TS_UNIX_AUTO_S_MAX => Self::Seconds,
            Self::TS_UNIX_AUTO_MS_MIN..=Self::TS_UNIX_AUTO_MS_MAX => Self::Milliseconds,
            Self::TS_UNIX_AUTO_US_MIN..=Self::TS_UNIX_AUTO_US_MAX => Self::Microseconds,
            _ => Self::Nanoseconds,
        }
    }

    const TS_UNIX_AUTO_S_MIN: i64 = -62135596800;
    const TS_UNIX_AUTO_S_MAX: i64 = 253402300799;
    const TS_UNIX_AUTO_MS_MIN: i64 = Self::TS_UNIX_AUTO_S_MIN * 1000;
    const TS_UNIX_AUTO_MS_MAX: i64 = Self::TS_UNIX_AUTO_S_MAX * 1000;
    const TS_UNIX_AUTO_US_MIN: i64 = Self::TS_UNIX_AUTO_MS_MIN * 1000;
    const TS_UNIX_AUTO_US_MAX: i64 = Self::TS_UNIX_AUTO_MS_MAX * 1000;
}
