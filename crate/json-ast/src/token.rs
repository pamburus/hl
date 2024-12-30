// external imports
use logos::Logos;

pub type Lexer<'s> = logos::Lexer<'s, Token<'s>>;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(error = crate::error::Error)]
pub enum Token<'s> {
    #[token("null")]
    Null,

    #[token("true", |_| true)]
    Bool(bool),

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| lex.slice())]
    Number(&'s str),

    #[regex(r#""[^"\\]*""#, |lex| lex.slice(), priority = 5)]
    PlainString(&'s str),

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#, |lex| lex.slice(), priority = 4)]
    EscapedString(&'s str),

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

pub fn lexer<'s>(source: &'s str) -> Lexer<'s> {
    Token::lexer(source)
}
