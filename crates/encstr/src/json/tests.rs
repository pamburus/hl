// third-party imports
use rstest::rstest;

// local imports
use super::*;

#[test]
fn test_decode_hex_val() {
    assert_eq!(decode_hex_val(b'0'), Some(0));
    assert_eq!(decode_hex_val(b'1'), Some(1));
    assert_eq!(decode_hex_val(b'2'), Some(2));
    assert_eq!(decode_hex_val(b'3'), Some(3));
    assert_eq!(decode_hex_val(b'4'), Some(4));
    assert_eq!(decode_hex_val(b'5'), Some(5));
    assert_eq!(decode_hex_val(b'6'), Some(6));
    assert_eq!(decode_hex_val(b'7'), Some(7));
    assert_eq!(decode_hex_val(b'8'), Some(8));
    assert_eq!(decode_hex_val(b'9'), Some(9));
    assert_eq!(decode_hex_val(b'A'), Some(10));
    assert_eq!(decode_hex_val(b'B'), Some(11));
    assert_eq!(decode_hex_val(b'C'), Some(12));
    assert_eq!(decode_hex_val(b'D'), Some(13));
    assert_eq!(decode_hex_val(b'E'), Some(14));
    assert_eq!(decode_hex_val(b'F'), Some(15));
    assert_eq!(decode_hex_val(b'G'), None);
    assert_eq!(decode_hex_val(b'g'), None);
    assert_eq!(decode_hex_val(b' '), None);
    assert_eq!(decode_hex_val(b'\n'), None);
    assert_eq!(decode_hex_val(b'\r'), None);
    assert_eq!(decode_hex_val(b'\t'), None);
}

#[test]
fn test_parser() {
    let mut result = Builder::new();
    let mut parser = Parser::new(r#""hello, \"world\"""#);
    parser.parse(&mut result).unwrap();
    assert_eq!(result.as_str(), "hello, \"world\"");
}

#[test]
fn test_tokens() {
    let mut tokens = Tokens::new(r#""hello, \"world\"""#);
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
    assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("world"))));
    assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
    assert_eq!(tokens.next(), None);
    assert_eq!(tokens.next(), None);
}

#[test]
fn test_tokens_escape() {
    let mut tokens = Tokens::new(r#""hello, \\\"world\"""#);
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
    assert_eq!(tokens.next(), Some(Ok(Token::Char('\\'))));
    assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("world"))));
    assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
    assert_eq!(tokens.next(), None);
    assert_eq!(tokens.next(), None);
}

#[test]
fn test_tokens_escape_b() {
    let mut tokens = Tokens::new(r#""00 \b""#);
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("00 "))));
    assert_eq!(tokens.next(), Some(Ok(Token::Char('\x08'))));
    assert_eq!(tokens.next(), None);
    assert_eq!(tokens.next(), None);
}

#[test]
fn test_tokens_control() {
    let mut tokens = Tokens::new(r#""hello, \x00world""#);
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
    assert_eq!(tokens.next(), Some(Err(Error::InvalidEscape)));
    assert_eq!(tokens.next(), Some(Err(Error::InvalidEscape)));
}

#[test]
fn test_tokens_eof() {
    let mut tokens = Tokens::new(r#""hello, \u"#);
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
    assert_eq!(tokens.next(), Some(Err(Error::Eof)));
    assert_eq!(tokens.next(), Some(Err(Error::Eof)));
}

#[test]
fn test_tokens_lone_surrogate() {
    let mut tokens = Tokens::new(r#""hello, \udc00world""#);
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
    assert_eq!(tokens.next(), Some(Err(Error::LoneLeadingSurrogateInHexEscape)));
}

#[test]
fn test_tokens_unexpected_end() {
    let mut tokens = Tokens::new(r#""hello, \ud800""#);
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
    assert_eq!(tokens.next(), Some(Err(Error::UnexpectedEndOfHexEscape)));
}

#[test]
fn test_tokens_invalid_surrogate_pair() {
    // Test case where first surrogate is followed by invalid second surrogate
    // This should trigger the lone leading surrogate error
    let mut tokens = Tokens::new(r#""\ud800\u1234""#);
    assert_eq!(tokens.next(), Some(Err(Error::LoneLeadingSurrogateInHexEscape)));
}

#[test]
fn test_append_esc_q() {
    let mut tokens = Tokens::new(r#""hello\u002c \"world\"""#);
    let mut buffer = Vec::new();
    let mut appender = Appender::new(&mut buffer);
    while let Some(Ok(token)) = tokens.next() {
        appender.handle(token);
    }
    assert_eq!(buffer, "hello, \\\"world\\\"".as_bytes());
}

#[test]
fn test_append_esc_bfnrt() {
    let mut tokens = Tokens::new(r#""00 \b\f\n\r\t""#);
    let mut buffer = Vec::new();
    let mut appender = Appender::new(&mut buffer);
    while let Some(Ok(token)) = tokens.next() {
        appender.handle(token);
    }
    assert_eq!(buffer, r#"00 \b\f\n\r\t"#.as_bytes());
}

#[test]
fn test_append_esc_unicode() {
    let mut tokens = Tokens::new(r#""00 ∞ \u2023""#);
    let mut buffer = Vec::new();
    let mut appender = Appender::new(&mut buffer);
    while let Some(Ok(token)) = tokens.next() {
        appender.handle(token);
    }
    assert_eq!(buffer, r#"00 ∞ ‣"#.as_bytes(), "{:?}", String::from_utf8_lossy(&buffer));
}

#[test]
fn test_append_sequence_with_quotes() {
    let mut buffer = Vec::new();
    let mut appender = Appender::new(&mut buffer);
    appender.handle(Token::Sequence(r#"hello, "world""#));
    assert_eq!(buffer, r#"hello, \"world\""#.as_bytes());
}

#[test]
#[should_panic]
fn test_invalid_json_string_empty() {
    JsonEncodedString::new("");
}

#[test]
#[should_panic]
fn test_invalid_json_single_quote() {
    JsonEncodedString::new(r#"""#);
}

#[test]
#[should_panic]
fn test_invalid_json_string_no_quotes() {
    JsonEncodedString::new("hello, world");
}

#[test]
#[should_panic]
fn test_invalid_json_string_no_closing_quote() {
    JsonEncodedString::new(r#""hello, world"#);
}

#[test]
#[should_panic]
fn test_invalid_json_string_no_opening_quote() {
    JsonEncodedString::new(r#"hello, world""#);
}

#[rstest]
fn test_find_escape(
    #[values(0x00, 0x09, 0x1f, b'"', b'\\', 0x7f)] escape: u8,
    #[values(b'a', b' ', b'!', b'#', b'[', b']', b'~', 0x80, 0xff)] filler: u8,
    #[values(0, 1, 7, 8, 9, 15, 16, 17)] pos: usize,
    #[values(1, 8, 18)] tail: usize,
) {
    let mut bytes = vec![filler; pos + tail];
    bytes[pos] = escape;
    assert_eq!(find_escape(&bytes), Some(pos));

    bytes[pos + tail - 1] = escape;
    assert_eq!(find_escape(&bytes), Some(pos));
}

#[rstest]
fn test_find_escape_none(
    #[values(b'a', b' ', b'!', b'#', b'[', b']', b'~', 0x80, 0xff)] filler: u8,
    #[values(0, 1, 7, 8, 9, 16, 17)] len: usize,
) {
    assert_eq!(find_escape(&vec![filler; len]), None);
}

#[test]
fn test_find_escape_all_bytes() {
    let found: Vec<u8> = (0..=u8::MAX).filter(|&b| find_escape(&[b; 8]).is_some()).collect();
    let expected: Vec<u8> = (0x00..=0x1f).chain([b'"', b'\\', 0x7f]).collect();
    assert_eq!(found, expected);
}

#[test]
fn test_append_sequence_long() {
    let mut buffer = Vec::new();
    let mut appender = Appender::new(&mut buffer);
    appender.handle(Token::Sequence("0123456789 \"quoted\"\tand\x7f∞ more text here\\"));
    assert_eq!(
        buffer,
        r#"0123456789 \"quoted\"\tand\u007f∞ more text here\\"#.as_bytes(),
        "{:?}",
        String::from_utf8_lossy(&buffer)
    );
}

#[rstest]
#[case::escapes(r#""aA\n\/""#, "aA\n/")]
#[case::surrogate_pair(r#""😀""#, "😀")]
#[case::lone_high_surrogate(r#""a\ud800b""#, "a\u{fffd}b")]
#[case::lone_low_surrogate(r#""\udc00""#, "\u{fffd}")]
#[case::high_surrogate_then_ascii(r#""\ud800A""#, "\u{fffd}A")]
#[case::high_surrogate_then_non_bmp(r#""\ud800𐀀""#, "\u{fffd}𐀀")]
#[case::high_surrogate_then_invalid_escape(r#""\ud800\x""#, "\u{fffd}\u{fffd}")]
#[case::invalid_escape(r#""\q""#, "\u{fffd}")]
fn test_decode_lossy(#[case] input: &str, #[case] expected: &str) {
    let mut result = Vec::new();
    JsonEncodedString::new(input).decode_lossy(&mut result).unwrap();
    assert_eq!(String::from_utf8(result).unwrap(), expected);
}

#[test]
fn test_decode_lossy_truncated_escape() {
    assert!(
        JsonEncodedString::new(r#""\u12""#)
            .decode_lossy(&mut Vec::new())
            .is_err()
    );
}
