use logos::Logos;

use upstream::{
    Span,
    token::{Scalar, String},
};

use super::ErrorKind;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(utf8 = false)]
#[logos(error = ErrorKind)]
pub enum Token {
    #[token("null", |_| Scalar::Null)]
    #[token("false", |_| Scalar::Bool(false))]
    #[token("true", |_| Scalar::Bool(true))]
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| Scalar::Number(lex.span().into()))]
    #[regex(r#""[^"\\\x00-\x1F]*""#, |lex| Scalar::String(String::Plain(unquote(lex.span().into()))), priority = 5)]
    #[regex(r#""([^"\\\x00-\x1F]|\\(["\\bnfrt/]|u[a-fA-F0-9]{4}))*""#, |lex| Scalar::String(String::JsonEscaped(lex.span().into())), priority = 4)]
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
fn unquote(s: Span) -> Span {
    (s.start + 1..s.end - 1).into()
}
