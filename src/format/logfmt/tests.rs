use encstr::{AnyEncodedString, EncodedString};

use super::{
    ast::{Composite, Scalar, Value},
    *,
};

#[test]
fn test_parse_all() {
    let input = r#"a=1 b=2 c=3"#;
    let mut lexer = Lexer::new(input);
    let container = parse_all(&mut lexer).unwrap();

    let mut roots = container.roots().iter();
    let root = roots.next().unwrap();
    assert!(roots.next().is_none());
    assert!(matches!(root.value(), Value::Composite(Composite::Object)));

    let mut children = root.children().iter();
    let key = children.next().unwrap();
    if let Value::Composite(Composite::Field(EncodedString::Raw(key))) = key.value() {
        assert_eq!(key.source(), "a");
    } else {
        panic!("expected field key");
    }
    let value = key.children().iter().next().unwrap();
    assert!(matches!(value.value(), Value::Scalar(Scalar::Number("1"))));

    let key = children.next().unwrap();
    assert!(matches!(
        key.value(),
        Value::Composite(Composite::Field(EncodedString::Raw(_)))
    ));
    let value = key.children().iter().next().unwrap();
    assert!(matches!(value.value(), Value::Scalar(Scalar::Number("2"))));

    let key = children.next().unwrap();
    assert!(matches!(
        key.value(),
        Value::Composite(Composite::Field(EncodedString::Raw(_)))
    ));
    let value = key.children().iter().next().unwrap();
    assert!(matches!(value.value(), Value::Scalar(Scalar::Number("3"))));

    assert!(children.next().is_none());
}
