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
