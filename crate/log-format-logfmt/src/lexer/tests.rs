use super::Lexer;
use upstream::token::{Composite::*, Scalar::*, String::*, Token::*};

macro_rules! next {
    ($expression:expr) => {
        (&mut $expression).next().unwrap().unwrap()
    };
}

#[test]
fn test_trivial_line() {
    let input = b"a=x";
    let mut lexer = Lexer::from_slice(input);
    assert_eq!(next!(lexer), EntryBegin);
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((0..1).into()))));
    assert_eq!(next!(lexer), Scalar(String(Plain((2..3).into()))));
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), EntryEnd);
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_two_lines() {
    let input = b"a=x\nb=y";
    let mut lexer = Lexer::from_slice(input);
    assert_eq!(next!(lexer), EntryBegin);
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((0..1).into()))));
    assert_eq!(next!(lexer), Scalar(String(Plain((2..3).into()))));
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), EntryEnd);
    assert_eq!(next!(lexer), EntryBegin);
    assert_eq!(next!(lexer), CompositeBegin(Field(Plain((4..5).into()))));
    assert_eq!(next!(lexer), Scalar(String(Plain((6..7).into()))));
    assert_eq!(next!(lexer), CompositeEnd);
    assert_eq!(next!(lexer), EntryEnd);
    assert_eq!(lexer.next(), None);
}
