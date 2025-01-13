use core::fmt::Display;

use upstream::Span;

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Error {
    pub kind: ErrorKind,
    pub span: Span,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} at {}", self.kind, self.span)
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum ErrorKind {
    #[default]
    InvalidToken,
    UnexpectedToken,
    UnexpectedEof,
    UnmatchedBrace,
    UnmatchedBracket,
    DepthLimitExceeded,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "invalid token"),
            Self::UnexpectedToken => write!(f, "unexpected token"),
            Self::UnexpectedEof => write!(f, "unexpected end of stream"),
            Self::UnmatchedBrace => write!(f, "unmatched opening brace"),
            Self::UnmatchedBracket => write!(f, "unmatched opening bracket"),
            Self::DepthLimitExceeded => write!(f, "depth limit exceeded"),
        }
    }
}

// ---

pub trait MakeError {
    fn make_error(&self, kind: ErrorKind) -> Error;
}

impl MakeError for logos::Span {
    #[inline]
    fn make_error(&self, kind: ErrorKind) -> Error {
        Error {
            kind,
            span: self.clone().into(),
        }
    }
}
