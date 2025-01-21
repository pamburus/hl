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

impl upstream::ast2::Error for Error {
    #[inline]
    fn kind(&self) -> upstream::ast2::ErrorKind {
        match self.kind {
            ErrorKind::InvalidToken => upstream::ast2::ErrorKind::InvalidToken,
            ErrorKind::ExpectedObject => upstream::ast2::ErrorKind::UnexpectedToken,
            ErrorKind::UnexpectedToken => upstream::ast2::ErrorKind::UnexpectedToken,
            ErrorKind::UnexpectedEof => upstream::ast2::ErrorKind::UnexpectedEof,
            ErrorKind::UnmatchedBrace => upstream::ast2::ErrorKind::UnmatchedTokenPair,
            ErrorKind::UnmatchedBracket => upstream::ast2::ErrorKind::UnmatchedTokenPair,
            ErrorKind::DepthLimitExceeded => upstream::ast2::ErrorKind::DepthLimitExceeded,
        }
    }

    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum ErrorKind {
    #[default]
    InvalidToken,
    ExpectedObject,
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
            Self::ExpectedObject => write!(f, "expected object"),
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
