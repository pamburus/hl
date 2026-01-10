// std imports
use std::borrow::Cow;
use std::boxed::Box;
use std::fmt;
use std::io;
use std::num::{ParseFloatError, ParseIntError, TryFromIntError};
use std::path::PathBuf;
use std::sync::mpsc;

// third-party imports
use config::ConfigError;
use owo_colors::OwoColorize;
use thiserror::Error;

// other local crates
use serde_logfmt::logfmt;

// local imports
use crate::level;
use crate::themecfg;
use crate::xerr::{Highlight, HighlightQuoted, Suggestions};

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    SizeParseError(#[from] SizeParseError),
    #[error(transparent)]
    NonZeroSizeParseError(#[from] NonZeroSizeParseError),
    #[error("failed to load configuration: {0}")]
    Config(Box<ConfigError>),
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    #[error(transparent)]
    Capnp(#[from] capnp::Error),
    #[error(transparent)]
    CiboriumSerialize(#[from] ciborium::ser::Error<std::io::Error>),
    #[error(transparent)]
    CiboriumDeserialize(#[from] ciborium::de::Error<std::io::Error>),
    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error + std::marker::Send>),
    #[error("file {filename:?} not found")]
    FileNotFoundError { filename: String },
    #[error(transparent)]
    InvalidLevel(#[from] InvalidLevelError),
    #[error("cannot recognize time {0:?}")]
    UnrecognizedTime(String),
    #[error(transparent)]
    Theme(#[from] themecfg::Error),
    #[error("failed to parse utf-8 string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("failed to construct utf-8 string from bytes: {0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("failed to parse yaml: {0}")]
    YamlError(#[from] yaml::serde::SerdeError),
    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
    #[error("failed to parse json: {0}")]
    WrongFieldFilter(String),
    #[error("wrong regular expression: {0}")]
    WrongRegularExpression(#[from] regex::Error),
    #[error("inconsistent index: {details}")]
    InconsistentIndex { details: String },
    #[error("failed to open file '{}' for reading: {source}", .path.hlq())]
    FailedToOpenFileForReading {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to open file '{}' for writing: {source}", .path.hlq())]
    FailedToOpenFileForWriting {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to get metadata of file '{}': {source}", .path.hlq())]
    FailedToGetFileMetadata {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to read file '{}': {source}", .path.hlq())]
    FailedToReadFile {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("failed to load file '{}': {source}", .path.hlq())]
    FailedToLoadFile {
        path: String,
        #[source]
        source: Box<Error>,
    },
    #[error("failed to parse json line {}: {source}", .line.hl())]
    FailedToParseJsonLine {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid index header")]
    InvalidIndexHeader,
    #[error("failed to parse json: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("failed to parse logfmt: {0}")]
    LogfmtParseError(#[from] logfmt::error::Error),
    #[error(transparent)]
    TryFromIntError(#[from] TryFromIntError),
    #[error(transparent)]
    NotifyError(Box<notify::Error>),
    #[error("failed to receive from mpsc channel: {source}")]
    RecvTimeoutError {
        #[source]
        source: mpsc::RecvTimeoutError,
    },
    #[error("failed to parse query:\n{0}")]
    QueryParseError(Box<pest::error::Error<crate::query::Rule>>),
    #[error(transparent)]
    LevelParseError(#[from] level::ParseError),
    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error("failed to detect application directories")]
    AppDirs,
}

impl Error {
    fn tips<'a, A>(&'a self, app: &A) -> Tips<'a>
    where
        A: AppInfoProvider,
    {
        match self {
            Error::Theme(themecfg::Error::ThemeNotFound { suggestions, .. }) => {
                let did_you_mean = did_you_mean(suggestions);
                let usage =
                    usage(app, UsageRequest::ListThemes).map(|usage| format!("run {usage} to list available themes"));
                Tips { did_you_mean, usage }
            }
            Error::Theme(themecfg::Error::ThemeOverlayNotFound { suggestions, .. }) => {
                let did_you_mean = did_you_mean(suggestions);
                let usage = usage(app, UsageRequest::ListThemeOverlays)
                    .map(|usage| format!("run {usage} to list available theme overlays"));
                Tips { did_you_mean, usage }
            }
            Error::LevelParseError(e) => {
                let did_you_mean = did_you_mean(&e.suggestions);
                Tips {
                    did_you_mean,
                    usage: None,
                }
            }
            _ => Default::default(),
        }
    }

    pub fn log<A>(&self, app: &A)
    where
        A: AppInfoProvider,
    {
        self.log_to(&mut io::stderr(), app).ok();
    }

    pub fn log_to<A, W>(&self, target: &mut W, app: &A) -> io::Result<()>
    where
        A: AppInfoProvider,
        W: std::io::Write,
    {
        writeln!(target, "{} {:#}", ERR_PREFIX.bright_red().bold(), self)?;
        write!(target, "{}", self.tips(app))?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct Tips<'a> {
    did_you_mean: Option<DidYouMean<'a>>,
    usage: Option<String>,
}

impl std::fmt::Display for Tips<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let prefix = TIP_PREFIX.green();
        let prefix = prefix.bold();

        if let Some(did_you_mean) = &self.did_you_mean {
            writeln!(f, "{prefix} {did_you_mean}")?;
        }

        if let Some(usage) = &self.usage {
            writeln!(f, "{prefix} {usage}")?;
        }

        Ok(())
    }
}

/// SizeParseError is an error which may occur when parsing size.
#[derive(Error, Debug)]
pub enum SizeParseError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error(transparent)]
    TryFromIntError(#[from] TryFromIntError),
    #[error(
        "invalid size {0:?}, use {iec0:?} or {iec1:?} format for IEC units or {si:?} format for SI units",
        iec0 = "64K",
        iec1 = "64KiB",
        si = "64KB"
    )]
    InvalidSize(String),
}

/// NonZeroSizeParseError is an error which may occur when parsing non-zero size.
#[derive(Error, Debug)]
pub enum NonZeroSizeParseError {
    #[error(transparent)]
    SizeParseError(#[from] SizeParseError),
    #[error("zero size")]
    ZeroSize,
}

/// NonZeroSizeParseError is an error which may occur when parsing non-zero size.
#[derive(Error, Debug)]
#[error("invalid level {value:?}, use any of {valid_values:?}")]
pub struct InvalidLevelError {
    pub value: String,
    pub valid_values: Vec<String>,
}

/// Result is an alias for standard result with bound Error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl From<ConfigError> for Error {
    fn from(err: ConfigError) -> Self {
        Error::Config(Box::new(err))
    }
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Error::NotifyError(Box::new(err))
    }
}

pub trait AppInfoProvider {
    fn app_name(&self) -> Cow<'static, str> {
        std::env::args().nth(0).map(Cow::Owned).unwrap_or("<app>".into())
    }

    fn usage_suggestion(&self, _request: UsageRequest) -> Option<UsageResponse> {
        None
    }
}

pub enum UsageRequest {
    ListThemes,
    ListThemeOverlays,
}

pub type UsageResponse = (Cow<'static, str>, Cow<'static, str>);

fn usage<A: AppInfoProvider>(app: &A, request: UsageRequest) -> Option<String> {
    let (command, args) = app.usage_suggestion(request)?;
    let result = format!("{} {}", app.app_name(), command);
    let result = result.bold();
    if args.is_empty() {
        Some(result.to_string())
    } else {
        Some(format!("{} {}", result, args))
    }
}

#[derive(Debug)]
pub struct DidYouMean<'a> {
    suggestions: &'a Suggestions,
}

impl fmt::Display for DidYouMean<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "did you mean ")?;
        for (i, suggestion) in self.suggestions.iter().enumerate() {
            if i > 0 {
                write!(f, " or ")?;
            }
            write!(f, "{}", suggestion.hlq())?;
        }
        write!(f, "?")
    }
}

fn did_you_mean(suggestions: &Suggestions) -> Option<DidYouMean<'_>> {
    if suggestions.is_empty() {
        return None;
    }

    Some(DidYouMean { suggestions })
}

impl From<pest::error::Error<crate::query::Rule>> for Error {
    fn from(err: pest::error::Error<crate::query::Rule>) -> Self {
        Error::QueryParseError(Box::new(err))
    }
}

const ERR_PREFIX: &str = "error:";
const TIP_PREFIX: &str = "  tip:";

#[cfg(test)]
mod tests;
