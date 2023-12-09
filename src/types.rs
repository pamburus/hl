// std imports
use std::cmp::Ord;

// third-party imports
use serde::Deserialize;

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
