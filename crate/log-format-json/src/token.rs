use logos::Logos;

use upstream::{
    source::{ByteSlice, Source},
    token::{Scalar, String},
};

use super::ErrorKind;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(source = Source)]
#[logos(error = ErrorKind)]
pub enum Token {
    #[token("null", |_| Scalar::Null)]
    #[token("false", |_| Scalar::Bool(false))]
    #[token("true", |_| Scalar::Bool(true))]
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| Scalar::Number(lex.slice()))]
    #[regex(r#""[^"\\\x00-\x1F]*""#, |lex| Scalar::String(String::Plain(unquote(lex.slice()))), priority = 5)]
    #[regex(r#""([^"\\\x00-\x1F]|\\(["\\bnfrt/]|u[a-fA-F0-9]{4}))*""#, |lex| Scalar::String(String::JsonEscaped(lex.slice())), priority = 4)]
    Scalar(Scalar),

    #[token("{")]
    BraceOpen,

    #[token("}")]
    BraceClose,

    #[token("[")]
    BracketOpen,

    #[token("]")]
    BracketClose,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,
}

#[inline]
fn unquote(s: ByteSlice) -> ByteSlice {
    s.slice(1..s.len() - 1)
}
