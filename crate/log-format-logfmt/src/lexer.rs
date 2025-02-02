use upstream::{
    token::{Composite, String},
    Lex, Token,
};

use super::{error::MakeError, token::InnerToken, Error, ErrorKind};

// ---

pub type InnerLexer<'s> = logos::Lexer<'s, InnerToken>;

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
    context: Context,
}

impl<'s> Lexer<'s> {
    #[inline]
    pub fn new(inner: InnerLexer<'s>) -> Self {
        Self {
            inner,
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
    type Item = Result<Token, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.context == Context::Root {
            self.context = Context::Field;
            return Some(Ok(Token::EntryBegin));
        }

        while let Some(token) = self.inner.next() {
            match token {
                Ok(token) => match (token, self.context) {
                    (InnerToken::Key(key), Context::Field) => {
                        return Some(Ok(Token::CompositeBegin(Composite::Field(String::Plain(key)))));
                    }
                    (InnerToken::Scalar(scalar), Context::Field) => {
                        self.context = Context::Delimiter;
                        return Some(Ok(Token::Scalar(scalar)));
                    }
                    (InnerToken::Space, Context::Delimiter) => {
                        self.context = Context::Field;
                        continue;
                    }
                    (InnerToken::Eol, _) => {
                        return Some(Ok(Token::EntryEnd));
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

// ---

pub struct BitStack {
    bits: u128,
    len: u8,
}

impl BitStack {
    #[inline]
    pub fn new() -> Self {
        Self { bits: 0, len: 0 }
    }

    #[inline]
    pub fn push(&mut self, bit: bool) -> Option<()> {
        if self.len == 128 {
            return None;
        }

        self.bits = (self.bits << 1) | (bit as u128);
        self.len += 1;
        Some(())
    }

    #[inline]
    pub fn pop(&mut self) -> Option<bool> {
        if self.len == 0 {
            return None;
        }

        let result = (self.bits & 1) == 1;
        self.bits >>= 1;
        self.len -= 1;
        Some(result)
    }

    #[inline]
    pub fn peek(&self) -> Option<bool> {
        if self.len == 0 {
            return None;
        }

        Some((self.bits & 1) == 1)
    }

    #[inline]
    pub fn len(&self) -> u8 {
        self.len
    }
}
