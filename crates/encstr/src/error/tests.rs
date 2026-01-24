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
