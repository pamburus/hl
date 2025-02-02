use super::Lexer;
use upstream::token::{Composite::*, Scalar::*, String::*, Token::*};

macro_rules! next {
    ($expression:expr) => {
        (&mut $expression).next().unwrap().unwrap()
    };
}

#[test]
fn test_trivial_object() {
    let input = br#"{"a":{"b":true}}"#;
    let mut lexer = Lexer::from_slice(input);
    assert_eq!(next!(lexer), EntryBegin);
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((2..3).into()))));
    assert_eq!(next!(lexer), CompositeBegin(Object));
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((7..8).into()))));
    assert_eq!(next!(lexer), Scalar(Bool(true)));
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), EntryEnd);
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_simple_object() {
    let input = br#"{"a":{"b":true,"d":["e",42,null]}}"#;
    let mut lexer = Lexer::from_slice(input);
    assert_eq!(next!(lexer), EntryBegin);
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((2..3).into()))));
    assert_eq!(next!(lexer), CompositeBegin(Object));
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((7..8).into()))));
    assert_eq!(next!(lexer), Scalar(Bool(true)));
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((16..17).into()))));
    assert_eq!(next!(lexer), CompositeBegin(Array));
    assert_eq!(next!(lexer), Scalar(String(Plain((21..22).into()))));
    assert_eq!(next!(lexer), Scalar(Number((24..26).into())));
    assert_eq!(next!(lexer), Scalar(Null));
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), EntryEnd);
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_two_trivial_objects() {
    let input = br#"{"a":{"b":true}}{}"#;
    let mut lexer = Lexer::from_slice(input);
    assert_eq!(next!(lexer), EntryBegin);
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((2..3).into()))));
    assert_eq!(next!(lexer), CompositeBegin(Object));
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((7..8).into()))));
    assert_eq!(next!(lexer), Scalar(Bool(true)));
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), EntryEnd);
    assert_eq!(next!(lexer), EntryBegin);
    assert_eq!(next!(lexer), EntryEnd);
    assert_eq!(lexer.next(), None);
}
