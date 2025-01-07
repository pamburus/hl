// external imports
use logos::Logos;

use super::{Format, Parse, ParseError, ParseOutput, ParseResult};
use crate::{error::Error, model::v2::ast};

// ---

pub struct LogfmtFormat;

impl Format for LogfmtFormat {
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

        match parse_line(&mut self.lexer, target) {
            (target, Ok(true)) => Some(Ok(ParseOutput {
                span: start..self.lexer.span().end,
                target,
            })),
            (_, Ok(false)) => None,
            (target, Err(e)) => Some(Err(ParseError {
                error: Error::FailedToParseLogfmtInput {
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
pub enum Token<'s> {
    #[regex(r#"[^"\x00-\x20='(),;<>\[\]\\\^`{}|\x7F]+="#, |lex| &lex.slice()[..lex.slice().len()-1])]
    Key(&'s str),

    #[token("null")]
    Null,

    #[token("false", |_| false)]
    #[token("true", |_| true)]
    Bool(bool),

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| lex.slice(), priority = 6)]
    Number(&'s str),

    #[regex(r#"[^"\x00-\x20=]+"#, |lex| String::Plain(lex.slice()), priority = 5)]
    #[regex(r#""([^"\\\x00-\x1F]|\\["\\bnfrt/]|u[a-fA-F0-9]{4})*""#, |lex| String::Escaped(lex.slice()), priority = 4)]
    String(String<'s>),

    #[regex(r#"[\t ]+"#)]
    Space,

    #[regex(r"\r\n|\r|\n")]
    Eol,
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
            String::Plain(s) => ast::String::raw(s),
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

        fn refine(self, lexer: &Lexer) -> Result<Self::Output>;
    }

    impl<T> Refine for std::result::Result<T, ()> {
        type Output = T;

        #[inline]
        fn refine(self, lexer: &Lexer) -> Result<T> {
            match self {
                Ok(value) => Ok(value),
                Err(_) => Err(("unexpected characters or end of stream", lexer.span())),
            }
        }
    }
}

pub use parse::{parse_all, parse_all_into, parse_line};

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
            match parse_line(lexer, target) {
                (t, Ok(true)) => target = t,
                (t, Ok(false)) => return (t, Ok(())),
                (t, Err(e)) => return (t, Err(e)),
            }
        }
    }

    #[inline]
    pub fn parse_line<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> OptResultTuple<T> {
        let (target, key) = parse_key(lexer, target);
        let key = match key {
            Ok(Some(key)) => key,
            Ok(None) => return (target, Ok(false)),
            Err(e) => return (target, Err(e)),
        };

        let (target, result) = target.add_composite(Composite::Object, |mut target| {
            let mut key = key;
            loop {
                target = match target.add_composite(Composite::Field(String::Plain(key).into()), |target| {
                    parse_value(lexer, target)
                }) {
                    (target, Ok(())) => target,
                    (target, Err(e)) => return (target, Err(e)),
                };

                let Some(token) = lexer.next() else {
                    break;
                };

                let token = match token.refine(lexer) {
                    Ok(token) => token,
                    Err(e) => return (target, Err(e)),
                };
                if token == Token::Eol {
                    break;
                }
                let Token::Space = token else {
                    return (target, Err(("unexpected token here (context: line)", lexer.span())));
                };

                match parse_key(lexer, target) {
                    (t, Ok(Some(k))) => {
                        target = t;
                        key = k;
                    }
                    (t, Ok(None)) => {
                        target = t;
                        break;
                    }
                    (t, Err(e)) => return (t, Err(e)),
                }
            }
            (target, Ok(()))
        });

        return (target, result.map(|_| true));
    }

    #[inline]
    fn parse_key<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> ResultTuple<T, Option<&'s str>> {
        loop {
            let Some(token) = lexer.next() else {
                return (target, Ok(None));
            };

            let token = match token.refine(lexer) {
                Ok(token) => token,
                Err(e) => return (target, Err(e)),
            };
            match token {
                Token::Space => continue,
                Token::Key(key) => return (target, Ok(Some(key))),
                _ => return (target, Err(("expected key here", lexer.span()))),
            }
        }
    }

    #[inline]
    fn parse_value<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> ResultTuple<T> {
        let Some(token) = lexer.next() else {
            return (
                target,
                Err(("unexpected end of stream while expecting field value", lexer.span())),
            );
        };

        let token = match token.refine(lexer) {
            Ok(token) => token,
            Err(e) => return (target, Err(e)),
        };

        let target = match token {
            Token::Null => target.add_scalar(Scalar::Null),
            Token::Bool(b) => target.add_scalar(Scalar::Bool(b)),
            Token::Number(s) => target.add_scalar(Scalar::Number(s)),
            Token::String(s) => target.add_scalar(Scalar::String(s.into())),
            _ => return (target, Err(("expected value here", lexer.span()))),
        };

        (target, Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use encstr::{AnyEncodedString, EncodedString};

    use super::{
        ast::{Composite, Scalar, Value},
        *,
    };

    #[test]
    fn test_parse_all() {
        let input = r#"a=1 b=2 c=3"#;
        let mut lexer = Lexer::new(input);
        let container = parse_all(&mut lexer).unwrap();

        let mut roots = container.roots().iter();
        let root = roots.next().unwrap();
        assert!(roots.next().is_none());
        assert!(matches!(root.value(), Value::Composite(Composite::Object)));

        let mut children = root.children().iter();
        let key = children.next().unwrap();
        if let Value::Composite(Composite::Field(EncodedString::Raw(key))) = key.value() {
            assert_eq!(key.source(), "a");
        } else {
            panic!("expected field key");
        }
        let value = key.children().iter().next().unwrap();
        assert!(matches!(value.value(), Value::Scalar(Scalar::Number("1"))));

        let key = children.next().unwrap();
        assert!(matches!(
            key.value(),
            Value::Composite(Composite::Field(EncodedString::Raw(_)))
        ));
        let value = key.children().iter().next().unwrap();
        assert!(matches!(value.value(), Value::Scalar(Scalar::Number("2"))));

        let key = children.next().unwrap();
        assert!(matches!(
            key.value(),
            Value::Composite(Composite::Field(EncodedString::Raw(_)))
        ));
        let value = key.children().iter().next().unwrap();
        assert!(matches!(value.value(), Value::Scalar(Scalar::Number("3"))));

        assert!(children.next().is_none());
    }
}
