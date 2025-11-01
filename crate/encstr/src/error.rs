use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Eof,
    InvalidEscape,
    LoneLeadingSurrogateInHexEscape,
    UnexpectedEndOfHexEscape,
    InvalidUnicodeCodePoint,
    UnexpectedControlCharacter,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Eof => f.write_str("unexpected end of input"),
            Self::InvalidEscape => f.write_str("invalid escape sequence"),
            Self::LoneLeadingSurrogateInHexEscape => f.write_str("lone leading surrogate in hex escape"),
            Self::UnexpectedEndOfHexEscape => f.write_str("unexpected end of hex escape"),
            Self::InvalidUnicodeCodePoint => f.write_str("invalid unicode code point"),
            Self::UnexpectedControlCharacter => f.write_str("unexpected control character"),
        }
    }
}

#[cfg(test)]
mod tests;
