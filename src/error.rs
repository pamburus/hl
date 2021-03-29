// std imports
use std::boxed::Box;
use std::io;
use std::num::{ParseIntError, TryFromIntError};

// third-party imports
use thiserror::Error;

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error(transparent)]
    TryFromIntError(#[from] TryFromIntError),
    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error + std::marker::Send>),
    #[error("file {filename:?} not found")]
    FileNotFoundError { filename: String },
    #[error("invalid level {value:?}, use any of {valid_values:?}")]
    InvalidLevel {
        value: String,
        valid_values: Vec<String>,
    },
    #[error(
        "invalid size {0:?}, use {:?} or {:?} format for IEC units or {:?} format for SI units",
        "64K",
        "64KiB",
        "64KB"
    )]
    InvalidSize(String),
    #[error("zero size")]
    ZeroSize,
}

/// Result is an alias for standard result with bound Error type.
pub type Result<T> = std::result::Result<T, Error>;
