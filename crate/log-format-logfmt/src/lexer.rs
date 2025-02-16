use upstream::{
    token::{Composite, Scalar, String},
    Lex, Source, Span,
};

use super::{error::MakeError, token::Token, Error, ErrorKind};

// ---

pub type InnerLexer<'s> = super::token::Lexer<'s>;

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

#[derive(Clone, Debug)]
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
    pub fn from_source(s: &'s Source) -> Self {
        Self::new(InnerLexer::new(s))
    }

    #[inline]
    pub fn span(&self) -> Span {
        Lex::span(self)
    }
}

impl<'s> Lex for Lexer<'s> {
    type Error = Error;

    #[inline]
    fn bump(&mut self, n: usize) {
        self.inner.bump(n);
    }

    #[inline]
    fn span(&self) -> Span {
        self.inner.span().into()
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Result<upstream::Token, Error>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match (&mut self.next, self.context) {
            (Some(Ok(Token::Key(key))), Context::Key) => {
                let key = String::Plain(std::mem::take(key));
                self.context = Context::Value;
                self.next = self.inner.next();
                Some(Ok(upstream::Token::CompositeBegin(Composite::Field(key))))
            }
            (Some(Ok(Token::Value(scalar))), Context::Value) => {
                self.context = Context::Delimiter;
                let scalar = std::mem::replace(scalar, Scalar::Null);
                self.next = self.inner.next();
                Some(Ok(upstream::Token::Scalar(scalar)))
            }
            (None | Some(Ok(Token::Eol)), Context::Delimiter) => {
                self.context = Context::Key;
                Some(Ok(upstream::Token::CompositeEnd))
            }
            (Some(Ok(Token::Space)), Context::Delimiter) => {
                self.context = Context::Key;
                self.next = self.inner.next();
                Some(Ok(upstream::Token::CompositeEnd))
            }
            (Some(Ok(Token::Space)), Context::Value) => {
                self.context = Context::Key;
                self.next = self.inner.next();
                let slice = self.inner.slice().slice(0..0);
                Some(Ok(upstream::Token::Scalar(Scalar::String(String::Plain(slice)))))
            }
            (None | Some(Ok(Token::Eol)), Context::Key) => {
                self.context = Context::Root;
                self.next = self.inner.next();
                Some(Ok(upstream::Token::EntryEnd))
            }
            (None, Context::Root) => None,
            (Some(Ok(_)), Context::Root) => {
                self.context = Context::Key;
                Some(Ok(upstream::Token::EntryBegin))
            }
            (&mut Some(Err(e)), _) => {
                self.next = self.inner.next();
                Some(Err(self.inner.make_error(e)))
            }
            (Some(Ok(_)), Context::Key) => Some(Err(self.inner.make_error(ErrorKind::ExpectedKey))),
            (None | Some(Ok(_)), Context::Value) => Some(Err(self.inner.make_error(ErrorKind::ExpectedValue))),
            (Some(Ok(_)), Context::Delimiter) => Some(Err(self.inner.make_error(ErrorKind::ExpectedSpace))),
        }
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Context {
    Root,
    Key,
    Value,
    Delimiter,
}

// ---

#[cfg(test)]
mod tests {
    use super::Lexer;
    use upstream::token::{Composite::*, Scalar::*, String::*, Token::*};

    macro_rules! next {
        ($expression:expr) => {
            (&mut $expression).next().unwrap().unwrap()
        };
    }

    #[test]
    fn test_trivial_line() {
        let input = b"a=x".into();
        let mut lexer = Lexer::from_source(&input);
        assert_eq!(next!(lexer), EntryBegin);
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((0..1).into()))));
        assert_eq!(next!(lexer), Scalar(String(Plain((2..3).into()))));
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), EntryEnd);
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_two_lines() {
        let input = b"a=x\nb=y".into();
        let mut lexer = Lexer::from_source(&input);
        assert_eq!(next!(lexer), EntryBegin);
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((0..1).into()))));
        assert_eq!(next!(lexer), Scalar(String(Plain((2..3).into()))));
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), EntryEnd);
        assert_eq!(next!(lexer), EntryBegin);
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((4..5).into()))));
        assert_eq!(next!(lexer), Scalar(String(Plain((6..7).into()))));
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), EntryEnd);
        assert_eq!(lexer.next(), None);
    }
}
