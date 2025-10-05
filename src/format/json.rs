// external imports
use logos::Logos;

use super::{Parse, ParseError, ParseOutput, ParseResult};
use crate::{error::Error, model::v2::ast};

// ---

pub struct JsonFormat;

impl super::Format for JsonFormat {
    type Lexer<'s> = Lexer<'s>;
    type Parser<'s> = Parser<'s>;

    fn new_lexer<'a>(&self, input: &'a [u8]) -> super::Result<Self::Lexer<'a>> {
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
            (target, Ok(true)) => Some(Ok(ParseOutput {
                span: start..self.lexer.span().end,
                target,
            })),
            (_, Ok(false)) => None,
            (target, Err(e)) => Some(Err(ParseError {
                error: Error::FailedToParseJsonInput {
                    message: e.0,
                    start: e.1.start,
                    end: e.1.end,
                },
                span: start..self.lexer.span().end,
                target,
            })),
        }
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
    #[regex(r#""([^"\\\x00-\x1F]|\\(["\\bnfrt/]|u[a-fA-F0-9]{4}))*""#, |lex| String::Escaped(lex.slice()), priority = 4)]
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
    pub type Result<T> = std::result::Result<T, Error>;
    pub type ResultTuple<T, R = ()> = (T, std::result::Result<R, Error>);
    pub type OptResultTuple<T> = ResultTuple<T, bool>;

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
        parse_all_into(lexer, container.metaroot()).1?;
        Ok(container)
    }

    #[inline]
    pub fn parse_all_into<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> ResultTuple<T> {
        loop {
            match parse_value(lexer, target) {
                (t, Ok(true)) => target = t,
                (t, Ok(false)) => return (t, Ok(())),
                (t, Err(e)) => return (t, Err(e)),
            }
        }
    }

    #[inline]
    pub fn parse_value<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> OptResultTuple<T> {
        if let Some(token) = lexer.next() {
            let token = match token.refine(lexer) {
                Ok(token) => token,
                Err(e) => return (target, Err(e)),
            };
            match parse_value_token(lexer, target, token) {
                (target, Ok(())) => (target, Ok(true)),
                (target, Err(e)) => (target, Err(e)),
            }
        } else {
            (target, Ok(false))
        }
    }

    #[inline]
    fn parse_field_value<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> ResultTuple<T> {
        match parse_value(lexer, target) {
            (target, Ok(true)) => (target, Ok(())),
            (target, Ok(false)) => (
                target,
                Err(("unexpected end of stream while expecting field value", lexer.span())),
            ),
            (target, Err(e)) => (target, Err(e)),
        }
    }

    #[inline]
    fn parse_value_token<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T, token: Token<'s>) -> ResultTuple<T> {
        match token {
            Token::Bool(b) => (target.add_scalar(Scalar::Bool(b)), Ok(())),
            Token::BraceOpen => {
                let mut skipped = true;
                let (target, result) = target.add_composite(Composite::Object, |target| {
                    skipped = false;
                    parse_object(lexer, target)
                });
                if let Err(e) = result {
                    return (target, Err(e));
                }
                if skipped {
                    if let Err(e) = parse_object(lexer, ast::Discarder::default()).1 {
                        return (target, Err(e));
                    }
                }
                (target, Ok(()))
            }
            Token::BracketOpen => {
                let mut skipped = true;
                let (target, result) = target.add_composite(Composite::Array, |target| {
                    skipped = false;
                    parse_array(lexer, target)
                });
                if let Err(e) = result {
                    return (target, Err(e));
                }
                if skipped {
                    if let Err(e) = parse_array(lexer, ast::Discarder::default()).1 {
                        return (target, Err(e));
                    }
                }
                (target, Ok(()))
            }
            Token::Null => (target.add_scalar(Scalar::Null), Ok(())),
            Token::Number(s) => (target.add_scalar(Scalar::Number(s)), Ok(())),
            Token::String(s) => (target.add_scalar(Scalar::String(s.into())), Ok(())),
            _ => (target, Err(("unexpected token here (context: value)", lexer.span()))),
        }
    }

    /// Parse a token stream into an array and return when
    /// a valid terminator is found.
    ///
    /// > NOTE: we assume '[' was consumed.
    #[inline]
    fn parse_array<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> ResultTuple<T> {
        let span = lexer.span();
        let mut awaits_comma = false;
        let mut awaits_value = false;

        while let Some(token) = lexer.next() {
            let token = match token.refine(lexer) {
                Ok(token) => token,
                Err(e) => return (target, Err(e)),
            };

            match token {
                Token::BracketClose if !awaits_value => {
                    return (target, Ok(()));
                }
                Token::Comma if awaits_comma => awaits_value = true,
                _ => {
                    match parse_value_token(lexer, target, token) {
                        (t, Ok(())) => target = t,
                        result => return result,
                    }
                    awaits_value = false;
                }
            }
            awaits_comma = !awaits_value;
        }
        (target, Err(("unmatched opening bracket defined here", span)))
    }

    /// Parse a token stream into an object and return when
    /// a valid terminator is found.
    ///
    /// > NOTE: we assume '{' was consumed.
    #[inline]
    fn parse_object<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> ResultTuple<T> {
        let span = lexer.span();

        enum Awaits {
            Key,
            Comma,
        }

        let mut awaits = Awaits::Key;

        while let Some(token) = lexer.next() {
            let token = match token.refine(lexer) {
                Ok(token) => token,
                Err(e) => return (target, Err(e)),
            };

            match (token, &mut awaits) {
                (Token::BraceClose, Awaits::Key | Awaits::Comma) => {
                    return (target, Ok(()));
                }
                (Token::Comma, Awaits::Comma) => {
                    awaits = Awaits::Key;
                }
                (Token::String(s), Awaits::Key) => {
                    match lexer.next() {
                        Some(Ok(Token::Colon)) => (),
                        _ => return (target, Err(("unexpected token here, expecting ':'", lexer.span()))),
                    }

                    let mut skipped = true;
                    let (t, result) = target.add_composite(Composite::Field(s.into()), |target| {
                        skipped = false;
                        parse_field_value(lexer, target)
                    });
                    target = t;
                    if let Err(e) = result {
                        return (target, Err(e));
                    }

                    if skipped {
                        match parse_field_value(lexer, ast::Discarder::default()).1 {
                            Ok(_) => (),
                            Err(e) => return (target, Err(e)),
                        }
                    }

                    awaits = Awaits::Comma;
                }
                _ => {
                    return (target, Err(("unexpected token here (context: object)", lexer.span())));
                }
            }
        }

        (target, Err(("unmatched opening brace defined here", span)))
    }
}
