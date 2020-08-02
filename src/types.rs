use std::cmp::Ord;

#[derive(Ord, PartialOrd, PartialEq, Eq, Debug, Hash, Clone)]
pub enum Level {
    Error,
    Warning,
    Info,
    Debug,
}
