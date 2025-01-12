use upstream::{
    token::{Composite, Scalar},
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
    stack: BitStack,
    context: Context,
}

impl<'s> Lexer<'s> {
    #[inline]
    pub fn new(inner: InnerLexer<'s>) -> Self {
        Self {
            inner,
            stack: BitStack::new(),
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
        if self.context == Context::FieldEnd {
            self.context = Context::ObjectDelimiter;
            return Some(Ok(Token::CompositeEnd));
        }

        while let Some(token) = self.inner.next() {
            match token {
                Ok(token) => match token {
                    InnerToken::Scalar(scalar) => match self.context {
                        Context::ArrayBegin | Context::ArrayNext => {
                            self.context = Context::ArrayDelimiter;
                            return Some(Ok(Token::Scalar(scalar)));
                        }
                        Context::ObjectBegin | Context::ObjectNext => {
                            if let Scalar::String(s) = scalar {
                                self.context = Context::FieldSeparator;
                                return Some(Ok(Token::CompositeBegin(Composite::Field(s))));
                            } else {
                                return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                            }
                        }
                        Context::FieldValue => {
                            self.context = Context::FieldEnd;
                            return Some(Ok(Token::Scalar(scalar)));
                        }
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    InnerToken::Comma => match self.context {
                        Context::ArrayDelimiter => {
                            self.context = Context::ArrayNext;
                            continue;
                        }
                        Context::ObjectDelimiter => {
                            self.context = Context::ObjectNext;
                            continue;
                        }
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    InnerToken::Colon => match self.context {
                        Context::FieldSeparator => {
                            self.context = Context::FieldValue;
                            continue;
                        }
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    InnerToken::BraceOpen => match self.context {
                        Context::Root
                        | Context::ArrayBegin
                        | Context::ArrayNext
                        | Context::ObjectBegin
                        | Context::FieldValue => match self.stack.push(false) {
                            Some(()) => {
                                self.context = Context::ObjectBegin;
                                if self.stack.len() == 1 {
                                    return Some(Ok(Token::EntryBegin));
                                }
                                return Some(Ok(Token::CompositeBegin(Composite::Object)));
                            }
                            None => return Some(Err(self.inner.make_error(ErrorKind::DepthLimitExceeded))),
                        },
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    InnerToken::BraceClose => match self.context {
                        Context::ObjectBegin | Context::ObjectDelimiter => match self.stack.pop() {
                            Some(false) => {
                                self.context = match self.stack.peek() {
                                    Some(false) => Context::FieldEnd,
                                    Some(true) => Context::ArrayDelimiter,
                                    None => Context::Root,
                                };
                                if self.context == Context::Root {
                                    return Some(Ok(Token::EntryEnd));
                                }
                                return Some(Ok(Token::CompositeEnd));
                            }
                            None | Some(true) => {
                                return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                            }
                        },
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    InnerToken::BracketOpen => match self.context {
                        Context::ArrayBegin | Context::ArrayNext | Context::FieldValue => match self.stack.push(true) {
                            Some(()) => {
                                self.context = Context::ArrayBegin;
                                return Some(Ok(Token::CompositeBegin(Composite::Array)));
                            }
                            None => return Some(Err(self.inner.make_error(ErrorKind::DepthLimitExceeded))),
                        },
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    InnerToken::BracketClose => match self.context {
                        Context::ArrayBegin | Context::ArrayDelimiter => match self.stack.pop() {
                            Some(true) => {
                                self.context = match self.stack.peek() {
                                    Some(false) => Context::FieldEnd,
                                    Some(true) => Context::ArrayDelimiter,
                                    None => Context::Root,
                                };
                                return Some(Ok(Token::CompositeEnd));
                            }
                            None | Some(false) => {
                                return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                            }
                        },
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
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
    ArrayBegin,
    ArrayDelimiter,
    ArrayNext,
    ObjectBegin,
    ObjectDelimiter,
    ObjectNext,
    FieldSeparator,
    FieldValue,
    FieldEnd,
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
