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

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum InputFormat {
    Json,
    Logfmt,
}
