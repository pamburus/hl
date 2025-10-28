// third-party imports
use assert_matches::assert_matches;
use chrono::TimeZone;
use maplit::hashmap;
use rstest::rstest;
use serde_logfmt::logfmt;

// local imports
use super::{ast::Build, *};
use crate::{
    error::Error,
    format,
    settings::{Field, FieldShowOption},
};

#[test]
fn test_value_is_empty() {
    let value = Value::Number("0");
    assert!(!value.is_empty());

    let value = Value::Number("123");
    assert!(!value.is_empty());

    let value = Value::String(EncodedString::raw(""));
    assert!(value.is_empty());

    let value = Value::String(EncodedString::raw("aa"));
    assert!(!value.is_empty());

    let value = Value::String(EncodedString::json(r#""""#));
    assert!(value.is_empty());

    let value = Value::String(EncodedString::json(r#""aa""#));
    assert!(!value.is_empty());

    let value = Value::Boolean(true);
    assert!(!value.is_empty());

    let value = Value::Null;
    assert!(value.is_empty());

    let mut container = ast::Container::new();

    let n = container.nodes().len();
    container.metaroot().add_composite(Composite::Object, |b| (b, Ok(())));
    let value = Value::Object(Object::new(container.nodes().get(n.into()).unwrap()));
    assert!(value.is_empty());

    let n = container.nodes().len();
    container
        .metaroot()
        .add_composite(Composite::Object, |b| (b.add_scalar(ast::Scalar::Number("1")), Ok(())));
    let value = Value::Object(Object::new(container.nodes().get(n.into()).unwrap()));
    assert!(!value.is_empty());

    let n = container.nodes().len();
    container.metaroot().add_composite(Composite::Array, |b| (b, Ok(())));
    let value = Value::Object(Object::new(container.nodes().get(n.into()).unwrap()));
    assert!(value.is_empty());

    let n = container.nodes().len();
    container
        .metaroot()
        .add_composite(Composite::Array, |b| (b.add_scalar(ast::Scalar::Number("1")), Ok(())));
    let value = Value::Array(Array::new(container.nodes().get(n.into()).unwrap()));
    assert!(!value.is_empty());
}
