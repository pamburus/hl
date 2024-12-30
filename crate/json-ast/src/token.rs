// external imports
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token<'s> {
    #[token("null")]
    Null,

    #[token("true", |_| true)]
    Bool(bool),

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| lex.slice())]
    Number(&'s str),

    #[regex(r#""[^"\\]*""#, |lex| lex.slice(), priority = 5)]
    RawString(&'s str),

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#, |lex| lex.slice(), priority = 4)]
    EncodedString(&'s str),

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
