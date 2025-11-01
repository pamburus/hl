use super::*;
use log_ast::ast::{Container, Value};
use upstream::{
    Span,
    ast::BuilderDetach,
    token::{Composite, Scalar, String},
};

#[test]
fn test_parse_line() {
    let input = br#"a=1 b=2 c=3"#;
    let mut lexer = Lexer::new(input);
    let mut container = Container::new();
    assert_eq!(
        parse_line(&mut lexer, container.metaroot()).detach().0.unwrap(),
        Some(Span::from(0..11))
    );

    let mut roots = container.roots().iter();
    let root = roots.next().unwrap();
    assert!(roots.next().is_none());
    assert!(matches!(root.value(), Value::Composite(Composite::Object)));

    let mut children = root.children().iter();
    let key = children.next().unwrap();
    if let &Value::Composite(Composite::Field(String::Plain(span))) = key.value() {
        assert_eq!(span, Span::from(0..1));
    } else {
        panic!("expected field key");
    }
    let value = key.children().iter().next().unwrap();
    assert!(matches!(
        value.value(),
        Value::Scalar(Scalar::Number(Span { start: 2, end: 3 }))
    ));

    let key = children.next().unwrap();
    assert!(matches!(
        key.value(),
        Value::Composite(Composite::Field(String::Plain(Span { start: 4, end: 5 })))
    ));
    let value = key.children().iter().next().unwrap();
    assert!(matches!(
        value.value(),
        Value::Scalar(Scalar::Number(Span { start: 6, end: 7 }))
    ));

    let key = children.next().unwrap();
    assert!(matches!(
        key.value(),
        Value::Composite(Composite::Field(String::Plain(Span { start: 8, end: 9 })))
    ));
    let value = key.children().iter().next().unwrap();
    assert!(matches!(
        value.value(),
        Value::Scalar(Scalar::Number(Span { start: 10, end: 11 }))
    ));

    assert!(children.next().is_none());
}
