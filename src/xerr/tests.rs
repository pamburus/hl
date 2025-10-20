use super::*;

#[test]
fn test_highlight() {
    assert_eq!("hello".hl().to_string(), "\u{1b}[33mhello\u{1b}[0m");
    assert_eq!(Path::new("hello").hl().to_string(), "\u{1b}[33mhello\u{1b}[0m");
    assert_eq!("hello".hlq().to_string(), "\u{1b}[33m\"hello\"\u{1b}[0m");
    assert_eq!(Path::new("hello").hlq().to_string(), "\u{1b}[33m\"hello\"\u{1b}[0m");
}
