// external imports
use logos::Logos;

use super::{Format, Parse, ParseOutput};
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

    fn parse<T: ast::Build<'s>>(&mut self, target: T) -> super::Result<Option<ParseOutput>> {
        parse_line(&mut self.lexer, target)
            .map_err(|e| Error::FailedToParseLogfmtInput {
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
        parse_all_into(lexer, container.metaroot())?;
        Ok(container)
    }

    #[inline]
    pub fn parse_all_into<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> Result<()> {
        while let Some(t) = parse_line(lexer, target)? {
            target = t;
        }
        Ok(())
    }

    #[inline]
    pub fn parse_line<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> Result<Option<T>> {
        let (mut target, key) = parse_key(lexer, target)?;
        let Some(key) = key else {
            return Ok(None);
        };

        target = target.add_composite(Composite::Object, |mut target| {
            let mut key = key;
            loop {
                target = target.add_composite(Composite::Field(String::Plain(key).into()), |target| {
                    parse_value(lexer, target)
                })?;

                let Some(token) = lexer.next() else {
                    break;
                };

                let token = token.refine(lexer)?;
                if token == Token::Eol {
                    break;
                }
                let Token::Space = token else {
                    return Err(("unexpected token here (context: line)", lexer.span()));
                };

                let (next_target, next_key) = parse_key(lexer, target)?;
                target = next_target;
                if let Some(k) = next_key {
                    key = k;
                } else {
                    break;
                }
            }
            Ok(target)
        })?;

        return Ok(Some(target));
    }

    #[inline]
    fn parse_key<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> Result<(T, Option<&'s str>)> {
        loop {
            let Some(token) = lexer.next() else {
                return Ok((target, None));
            };

            let token = token.refine(lexer)?;
            match token {
                Token::Space => continue,
                Token::Key(key) => return Ok((target, Some(key))),
                _ => return Err(("expected key here", lexer.span())),
            }
        }
    }

    #[inline]
    fn parse_value<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> Result<T> {
        let Some(token) = lexer.next() else {
            return Err(("unexpected end of stream while expecting field value", lexer.span()));
        };

        let token = token.refine(lexer)?;

        Ok(match token {
            Token::Null => target.add_scalar(Scalar::Null),
            Token::Bool(b) => target.add_scalar(Scalar::Bool(b)),
            Token::Number(s) => target.add_scalar(Scalar::Number(s)),
            Token::String(s) => target.add_scalar(Scalar::String(s.into())),
            _ => return Err(("expected value here", lexer.span())),
        })
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
