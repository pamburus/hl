use super::*;

#[test]
fn test_raw_string() {
    let mut result = Builder::new();
    let string = RawString::new("hello, world!¡");
    string.decode(&mut result).unwrap();
    assert_eq!(result.as_str(), "hello, world!¡");
}

#[test]
fn test_appender() {
    let mut buffer = Vec::new();
    let mut appender = Appender::new(&mut buffer);
    appender.handle(Token::Sequence("hello ")).unwrap();
    appender.handle(Token::Char('•')).unwrap();
    appender.handle(Token::Sequence(" world")).unwrap();
    assert_eq!(std::str::from_utf8(&buffer).unwrap(), "hello • world");
}
