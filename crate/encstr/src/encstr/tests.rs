use super::*;

#[test]
fn builder() {
    let mut result = Builder::new();
    result.handle(Token::Char('h')).unwrap();
    result.handle(Token::Char('e')).unwrap();
    result.handle(Token::Char('l')).unwrap();
    result.handle(Token::Char('l')).unwrap();
    result.handle(Token::Char('o')).unwrap();
    result.handle(Token::Char(',')).unwrap();
    result.handle(Token::Char(' ')).unwrap();
    result.handle(Token::Char('w')).unwrap();
    result.handle(Token::Char('o')).unwrap();
    result.handle(Token::Char('r')).unwrap();
    result.handle(Token::Char('l')).unwrap();
    result.handle(Token::Char('d')).unwrap();
    assert_eq!(result.as_str(), "hello, world");
}

#[test]
fn builder_default() {
    let builder1 = Builder::new();
    let builder2 = Builder::default();

    // Both should start with empty content
    assert_eq!(builder1.as_str(), "");
    assert_eq!(builder2.as_str(), "");

    // Both should have the same capacity
    assert_eq!(builder1.buffer.capacity(), builder2.buffer.capacity());
}

#[test]
fn encoded_string_raw() {
    let s = EncodedString::raw("hello, world!");
    let mut tokens = s.tokens();
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, world!"))));
    assert_eq!(tokens.next(), None);
    assert_eq!(tokens.next(), None);
}

#[test]
fn encoded_string_json() {
    let s = EncodedString::json(r#""hello, \"world\"!""#);
    let mut tokens = s.tokens();
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
    assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("world"))));
    assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
    assert_eq!(tokens.next(), Some(Ok(Token::Sequence("!"))));
    assert_eq!(tokens.next(), None);
    assert_eq!(tokens.next(), None);
}

#[test]
fn test_bytes_unicode_escape() {
    let s = EncodedString::json(r#""\u2023""#);
    let bytes: Result<Vec<u8>> = s.bytes().collect();
    // This should trigger UTF-8 encoding on line 201 for the ‣ character (U+2023)
    assert_eq!(bytes.unwrap(), "‣".as_bytes());
}
