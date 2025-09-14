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
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(Error::Eof.to_string(), "unexpected end of input");
        assert_eq!(Error::InvalidEscape.to_string(), "invalid escape sequence");
        assert_eq!(
            Error::LoneLeadingSurrogateInHexEscape.to_string(),
            "lone leading surrogate in hex escape"
        );
        assert_eq!(
            Error::UnexpectedEndOfHexEscape.to_string(),
            "unexpected end of hex escape"
        );
        assert_eq!(Error::InvalidUnicodeCodePoint.to_string(), "invalid unicode code point");
        assert_eq!(
            Error::UnexpectedControlCharacter.to_string(),
            "unexpected control character"
        );
    }
}
