// std imports
use std::sync::Arc;

// third-party imports
use thiserror::Error;

// local imports
use crate::xerr::{HighlightQuoted, Suggestions};

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid input info option {value}", value=.value.hlq())]
    InvalidInputInfo { value: Arc<str>, suggestions: Suggestions },
}
