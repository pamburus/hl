use logos::Logos;

use upstream::{
    token::{Scalar, String},
    Span,
};

use super::ErrorKind;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(source = [u8])]
#[logos(error = ErrorKind)]
pub enum Token {
    #[regex(r#"[^"\x00-\x20='(),;<>\[\]\\\^`{}|\x7F]+="#, |lex| Span::from(lex.span()).cut_right(1))]
    Key(Span),

    #[token("null", |_| Scalar::Null)]
    #[token("false", |_| Scalar::Bool(false))]
    #[token("true", |_| Scalar::Bool(true))]
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| Scalar::Number(lex.span().into()), priority = 6)]
    #[regex(r#"[^"\x00-\x20=]+"#, |lex| Scalar::String(String::Plain(lex.span().into())), priority = 5)]
    #[regex(r#""([^"\\\x00-\x1F]|\\(["\\bnfrt/]|u[a-fA-F0-9]{4}))*""#, |lex| Scalar::String(String::JsonEscaped(lex.span().into())), priority = 4)]
    Value(Scalar),

    #[regex(r#"[\t ]+"#)]
    Space,

    #[regex(r"\r\n|\r|\n")]
    Eol,
}
