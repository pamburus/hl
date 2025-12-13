use super::*;

#[test]
fn test_container() {
    let mut lexer = Lexer::new(r#"{"key": "value"}"#);
    let container = Container::parse(&mut lexer).unwrap();
    assert_eq!(container.nodes().len(), 4);
}
