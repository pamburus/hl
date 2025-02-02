use std::mem::replace;

use upstream::{
    token::{Composite, String},
    Lex,
};

use super::{error::MakeError, token::Token, Error, ErrorKind};

// ---

pub type InnerLexer<'s> = logos::Lexer<'s, Token>;

impl<'s> MakeError for InnerLexer<'s> {
    #[inline]
    fn make_error(&self, kind: ErrorKind) -> Error {
        Error {
            kind,
            span: self.span().into(),
        }
    }
}

// ---

pub struct Lexer<'s> {
    inner: InnerLexer<'s>,
    next: Option<Result<Token, ErrorKind>>,
    context: Context,
}

impl<'s> Lexer<'s> {
    #[inline]
    pub fn new(mut inner: InnerLexer<'s>) -> Self {
        let next = inner.next();
        Self {
            inner,
            next,
            context: Context::Root,
        }
    }

    #[inline]
    pub fn from_slice(s: &'s [u8]) -> Self {
        Self::new(InnerLexer::new(s))
    }
}

impl<'s> Lex for Lexer<'s> {
    type Error = Error;
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Result<upstream::Token, Error>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_none() {
            return None;
        }

        if self.context == Context::Root {
            self.context = Context::Field;
            return Some(Ok(upstream::Token::EntryBegin));
        }

        while let Some(token) = replace(&mut self.next, self.inner.next()) {
            match token {
                Ok(token) => match (token, self.context) {
                    (Token::Key(key), Context::Field) => {
                        let key = String::Plain(key);
                        return Some(Ok(upstream::Token::CompositeBegin(Composite::Field(key))));
                    }
                    (Token::Scalar(scalar), Context::Field) => {
                        self.context = Context::Delimiter;
                        return Some(Ok(upstream::Token::Scalar(scalar)));
                    }
                    (Token::Space, Context::Delimiter) => {
                        self.context = Context::Field;
                        continue;
                    }
                    (Token::Eol, _) => {
                        self.context = Context::Root;
                        return Some(Ok(upstream::Token::EntryEnd));
                    }
                    _ => return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken))),
                },
                Err(e) => return Some(Err(self.inner.make_error(e))),
            }
        }

        None
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Context {
    Root,
    Field,
    Delimiter,
}
