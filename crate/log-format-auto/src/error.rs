use core::fmt::Display;

use upstream::Span;

#[cfg(feature = "json")]
pub use log_format_json::{Error as JsonError, ErrorKind as JsonErrorKind};

#[cfg(feature = "logfmt")]
pub use log_format_logfmt::{Error as LogfmtError, ErrorKind as LogfmtErrorKind};

use crate::EnabledFormatList;

// ---

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub kind: ErrorKind,
    pub span: Span,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} at {}", self.kind, self.span)
    }
}

#[cfg(feature = "json")]
impl From<JsonError> for Error {
    fn from(e: JsonError) -> Self {
        Self {
            kind: e.kind.into(),
            span: e.span,
        }
    }
}

#[cfg(feature = "logfmt")]
impl From<LogfmtError> for Error {
    fn from(e: LogfmtError) -> Self {
        Self {
            kind: e.kind.into(),
            span: e.span,
        }
    }
}

// ---
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    CannotDetermineFormat(EnabledFormatList),
    #[cfg(feature = "json")]
    Json(JsonErrorKind),
    #[cfg(feature = "logfmt")]
    Logfmt(LogfmtErrorKind),
}

#[cfg(feature = "json")]
impl From<JsonErrorKind> for ErrorKind {
    fn from(e: JsonErrorKind) -> Self {
        Self::Json(e)
    }
}

#[cfg(feature = "logfmt")]
impl From<LogfmtErrorKind> for ErrorKind {
    fn from(e: LogfmtErrorKind) -> Self {
        Self::Logfmt(e)
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CannotDetermineFormat(formats) => {
                write!(f, "cannot determine format from {:?}", formats)
            }
            #[cfg(feature = "json")]
            Self::Json(e) => e.fmt(f),
            #[cfg(feature = "logfmt")]
            Self::Logfmt(e) => e.fmt(f),
        }
    }
}
