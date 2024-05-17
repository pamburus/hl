use std::{error, fmt};

use serde::de;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Eof,
    ExpectedBoolean,
    ExpectedInteger,
    ExpectedNull,
    ExpectedString,
    ExpectedKey,
    ExpectedKeyValueDelimiter,
    Syntax,
    InvalidEscape,
    LoneLeadingSurrogateInHexEscape,
    UnexpectedEndOfHexEscape,
    InvalidUnicodeCodePoint,
    TrailingCharacters,
    UnexpectedControlCharacter,
    UnexpectedByte(u8),
    Custom(String),
    InvalidUtf8(std::str::Utf8Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Eof => f.write_str("unexpected end of input"),
            Self::ExpectedBoolean => f.write_str("expected boolean"),
            Self::ExpectedInteger => f.write_str("expected integer"),
            Self::ExpectedNull => f.write_str("expected null"),
            Self::ExpectedString => f.write_str("expected string"),
            Self::ExpectedKey => f.write_str("expected key"),
            Self::ExpectedKeyValueDelimiter => f.write_str("expected key-value delimiter"),
            Self::Syntax => f.write_str("syntax error"),
            Self::InvalidEscape => f.write_str("invalid escape sequence"),
            Self::LoneLeadingSurrogateInHexEscape => f.write_str("lone leading surrogate in hex escape"),
            Self::UnexpectedEndOfHexEscape => f.write_str("unexpected end of hex escape"),
            Self::InvalidUnicodeCodePoint => f.write_str("invalid unicode code point"),
            Self::TrailingCharacters => f.write_str("trailing characters"),
            Self::UnexpectedControlCharacter => f.write_str("unexpected control character"),
            Self::UnexpectedByte(byte) => write!(f, "unexpected byte: {}", byte),
            Self::Custom(msg) => f.write_str(msg),
            Self::InvalidUtf8(err) => write!(f, "invalid utf-8: {}", err),
        }
    }
}

impl serde::de::StdError for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl de::Error for Error {
    #[cold]
    fn custom<T: fmt::Display>(msg: T) -> Error {
        Self::Custom(msg.to_string())
    }

    #[cold]
    fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        Error::custom(format_args!("invalid type: {}, expected {}", Unexpected(unexp), exp,))
    }

    #[cold]
    fn invalid_value(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        Error::custom(format_args!("invalid value: {}, expected {}", Unexpected(unexp), exp,))
    }
}

struct Unexpected<'a>(de::Unexpected<'a>);

impl<'a> fmt::Display for Unexpected<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            de::Unexpected::Unit => formatter.write_str("null"),
            de::Unexpected::Float(value) => write!(formatter, "floating point `{}`", value),
            unexp => fmt::Display::fmt(&unexp, formatter),
        }
    }
}
