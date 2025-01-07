// external imports
use logos::Logos;

use super::{Parse, ParseError, ParseOutput, ParseResult};
use crate::{
    error::Error,
    model::v2::ast,
};

// ---

pub struct JsonFormat;

impl super::Format for JsonFormat {
    type Lexer<'s> = Lexer<'s>;
    type Parser<'s> = Parser<'s>;

    fn new_lexer<'a>(&self, input: &'a [u8]) -> Result<Self::Lexer<'a>> {
        Ok(Lexer::new(std::str::from_utf8(input)?))
    }

    fn new_parser_from_lexer<'s>(&self, lexer: Self::Lexer<'s>) -> Self::Parser<'s> {
        Parser { lexer }
    }
}

// ---

pub struct Parser<'s> {
    lexer: Lexer<'s>,
}

impl<'s> Parse<'s> for Parser<'s> {
    type Lexer = Lexer<'s>;

    fn parse<T: ast::Build<'s>>(&mut self, target: T) -> ParseResult<T> {
        let start = self.lexer.span().start;

        match parse_value(&mut self.lexer, target) {
            Ok(Some(target)) => Ok(ParseOutput {
                span: start..self.lexer.span().end,
                target,
            }),
            Ok(None) => None,
            Err(e) => Err(ParseError {
                error: ParseError::FailedToParseJsonInput {
                    message: e.0,
                    start: e.1.start,
                    end: e.1.end,
                },
                span: start..self.lexer.span().end,
                target: 
            }),
        }
        .map_err(|e| Error::FailedToParseJsonInput {
            message: e.0,
            start: e.1.start,
            end: e.1.end,
        })
        .map(|x| {
            x.map(|_| ParseOutput {
                span: 0..self.lexer.span().end,
            })
        })
    }

    fn into_lexer(self) -> Self::Lexer {
        self.lexer
    }
}

// ---

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token<'s> {
    #[token("null")]
    Null,

    #[token("false", |_| false)]
    #[token("true", |_| true)]
    Bool(bool),

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| lex.slice())]
    Number(&'s str),

    #[regex(r#""[^"\\\x00-\x1F]*""#, |lex| String::Plain(lex.slice()), priority = 5)]
    #[regex(r#""([^"\\\x00-\x1F]|\\["\\bnfrt/]|u[a-fA-F0-9]{4})*""#, |lex| String::Escaped(lex.slice()), priority = 4)]
    String(String<'s>),

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

#[derive(Debug, PartialEq, Clone)]
pub enum String<'s> {
    Plain(&'s str),
    Escaped(&'s str),
}

impl<'s> Into<ast::String<'s>> for String<'s> {
    #[inline]
    fn into(self) -> ast::String<'s> {
        match self {
            String::Plain(s) => ast::String::raw(&s[1..s.len() - 1]),
            String::Escaped(s) => ast::String::json(s),
        }
    }
}

pub type Lexer<'s> = logos::Lexer<'s, Token<'s>>;

pub mod error {
    use super::Lexer;
    use logos::Span;

    pub type Error = (&'static str, Span);
    pub type Result<T> = std::result::Result<T, (T, Error)>;

    pub trait Refine {
        type Output;

        fn refine(self, lexer: &Lexer) -> std::result::Result<Self::Output, Error>;
    }

    impl<T> Refine for std::result::Result<T, ()> {
        type Output = T;

        #[inline]
        fn refine(self, lexer: &Lexer) -> std::result::Result<T, Error> {
            self.map_err(|_| ("unexpected characters or end of stream", lexer.span()))
        }
    }
}

pub use parse::{parse_all, parse_all_into, parse_value};

mod parse {
    use super::{error::*, *};
    use crate::model::v2::ast::{Build, Composite, Container, Scalar};

    #[inline]
    pub fn parse_all<'s>(lexer: &mut Lexer<'s>) -> Result<Container<'s>> {
        let mut container = Container::new();
        parse_all_into(lexer, container.metaroot())?;
        Ok(container)
    }

    #[inline]
    pub fn parse_all_into<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> Result<()> {
        while let Some(t) = parse_value(lexer, target)? {
            target = t;
        }
        Ok(())
    }

    #[inline]
    pub fn parse_value<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> Option<Result<T>> {
        if let Some(token) = lexer.next() {
            let token = match token.refine(lexer) {
                Ok(token) => token,
                Err(e) => return Some(Err((target, e))),
            };
            Some(parse_value_token(lexer, target, token))
        } else {
            None
        }
    }

    #[inline]
    fn parse_field_value<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> Result<T> {
        if let Some(target) = parse_value(lexer, target)? {
            Ok(target)
        } else {
            Err(("unexpected end of stream while expecting field value", lexer.span()))
        }
    }

    #[inline]
    fn parse_value_token<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T, token: Token<'s>) -> Result<T> {
        match token {
            Token::Bool(b) => Ok(target.add_scalar(Scalar::Bool(b))),
            Token::BraceOpen => {
                let mut skipped = true;
                let target = target.add_composite(Composite::Object, |target| {
                    skipped = false;
                    parse_object(lexer, target)
                })?;
                if skipped {
                    parse_object(lexer, ast::Discarder::default())?;
                }
                Ok(target)
            }
            Token::BracketOpen => {
                let mut skipped = true;
                let target = target.add_composite(Composite::Array, |target| {
                    skipped = false;
                    parse_array(lexer, target)
                })?;
                if skipped {
                    parse_array(lexer, ast::Discarder::default())?;
                }
                Ok(target)
            }
            Token::Null => Ok(target.add_scalar(Scalar::Null)),
            Token::Number(s) => Ok(target.add_scalar(Scalar::Number(s))),
            Token::String(s) => Ok(target.add_scalar(Scalar::String(s.into()))),
            _ => Err(("unexpected token here (context: value)", lexer.span())),
        }
    }

    /// Parse a token stream into an array and return when
    /// a valid terminator is found.
    ///
    /// > NOTE: we assume '[' was consumed.
    #[inline]
    fn parse_array<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> Result<T> {
        let span = lexer.span();
        let mut awaits_comma = false;
        let mut awaits_value = false;

        while let Some(token) = lexer.next() {
            let token = token.refine(lexer)?;
            match token {
                Token::BracketClose if !awaits_value => return Ok(target),
                Token::Comma if awaits_comma => awaits_value = true,
                _ => {
                    target = parse_value_token(lexer, target, token)?;
                    awaits_value = false;
                }
            }
            awaits_comma = !awaits_value;
        }
        Err(("unmatched opening bracket defined here", span))
    }

    /// Parse a token stream into an object and return when
    /// a valid terminator is found.
    ///
    /// > NOTE: we assume '{' was consumed.
    #[inline]
    fn parse_object<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> Result<T> {
        let span = lexer.span();

        enum Awaits {
            Key,
            Comma,
        }

        let mut awaits = Awaits::Key;

        while let Some(token) = lexer.next() {
            match (token.refine(lexer)?, &mut awaits) {
                (Token::BraceClose, Awaits::Key | Awaits::Comma) => {
                    return Ok(target);
                }
                (Token::Comma, Awaits::Comma) => {
                    awaits = Awaits::Key;
                }
                (Token::String(s), Awaits::Key) => {
                    match lexer.next() {
                        Some(Ok(Token::Colon)) => (),
                        _ => return Err(("unexpected token here, expecting ':'", lexer.span())),
                    }

                    let mut skipped = true;
                    target = target.add_composite(Composite::Field(s.into()), |target| {
                        skipped = false;
                        parse_field_value(lexer, target)
                    })?;

                    if skipped {
                        parse_field_value(lexer, ast::Discarder::default())?;
                    }

                    awaits = Awaits::Comma;
                }
                _ => {
                    return Err(("unexpected token here (context: object)", lexer.span()));
                }
            }
        }

        Err(("unmatched opening brace defined here", span))
    }
}
