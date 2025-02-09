use upstream::{
    token::{Composite, Scalar},
    Lex, Span,
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

#[derive(Clone, Debug)]
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

    #[inline]
    pub fn span(&self) -> Span {
        Lex::span(self)
    }
}

impl<'s> Lex for Lexer<'s> {
    type Error = Error;

    #[inline]
    fn span(&self) -> Span {
        self.inner.span().into()
    }

    #[inline]
    fn bump(&mut self, n: usize) {
        self.inner.bump(n);
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Result<upstream::Token, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.context == Context::FieldEnd {
            self.context = Context::ObjectDelimiter;
            return Some(Ok(upstream::Token::CompositeEnd));
        }

        while let Some(token) = self.inner.next() {
            match token {
                Ok(token) => match token {
                    Token::Scalar(scalar) => match self.context {
                        Context::ArrayBegin | Context::ArrayNext => {
                            self.context = Context::ArrayDelimiter;
                            return Some(Ok(upstream::Token::Scalar(scalar)));
                        }
                        Context::ObjectBegin | Context::ObjectNext => {
                            if let Scalar::String(s) = scalar {
                                self.context = Context::FieldSeparator;
                                return Some(Ok(upstream::Token::CompositeBegin(Composite::Field(s))));
                            } else {
                                return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                            }
                        }
                        Context::FieldValue => {
                            self.context = Context::FieldEnd;
                            return Some(Ok(upstream::Token::Scalar(scalar)));
                        }
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    Token::Comma => match self.context {
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
                    Token::Colon => match self.context {
                        Context::FieldSeparator => {
                            self.context = Context::FieldValue;
                            continue;
                        }
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    Token::BraceOpen => match self.context {
                        Context::Root
                        | Context::ArrayBegin
                        | Context::ArrayNext
                        | Context::ObjectBegin
                        | Context::FieldValue => match self.stack.push(false) {
                            Some(()) => {
                                self.context = Context::ObjectBegin;
                                if self.stack.len() == 1 {
                                    return Some(Ok(upstream::Token::EntryBegin));
                                }
                                return Some(Ok(upstream::Token::CompositeBegin(Composite::Object)));
                            }
                            None => return Some(Err(self.inner.make_error(ErrorKind::DepthLimitExceeded))),
                        },
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    Token::BraceClose => match self.context {
                        Context::ObjectBegin | Context::ObjectDelimiter => match self.stack.pop() {
                            Some(false) => {
                                self.context = match self.stack.peek() {
                                    Some(false) => Context::FieldEnd,
                                    Some(true) => Context::ArrayDelimiter,
                                    None => Context::Root,
                                };
                                if self.context == Context::Root {
                                    return Some(Ok(upstream::Token::EntryEnd));
                                }
                                return Some(Ok(upstream::Token::CompositeEnd));
                            }
                            None | Some(true) => {
                                return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                            }
                        },
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    Token::BracketOpen => match self.context {
                        Context::ArrayBegin | Context::ArrayNext | Context::FieldValue => match self.stack.push(true) {
                            Some(()) => {
                                self.context = Context::ArrayBegin;
                                return Some(Ok(upstream::Token::CompositeBegin(Composite::Array)));
                            }
                            None => return Some(Err(self.inner.make_error(ErrorKind::DepthLimitExceeded))),
                        },
                        _ => {
                            return Some(Err(self.inner.make_error(ErrorKind::UnexpectedToken)));
                        }
                    },
                    Token::BracketClose => match self.context {
                        Context::ArrayBegin | Context::ArrayDelimiter => match self.stack.pop() {
                            Some(true) => {
                                self.context = match self.stack.peek() {
                                    Some(false) => Context::FieldEnd,
                                    Some(true) => Context::ArrayDelimiter,
                                    None => Context::Root,
                                };
                                return Some(Ok(upstream::Token::CompositeEnd));
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

#[derive(Debug, Clone)]
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
    fn test_trivial_object() {
        let input = br#"{"a":{"b":true}}"#;
        let mut lexer = Lexer::from_slice(input);
        assert_eq!(next!(lexer), EntryBegin);
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((2..3).into()))));
        assert_eq!(next!(lexer), CompositeBegin(Object));
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((7..8).into()))));
        assert_eq!(next!(lexer), Scalar(Bool(true)));
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), EntryEnd);
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_simple_object() {
        let input = br#"{"a":{"b":true,"d":["e",42,null]}}"#;
        let mut lexer = Lexer::from_slice(input);
        assert_eq!(next!(lexer), EntryBegin);
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((2..3).into()))));
        assert_eq!(next!(lexer), CompositeBegin(Object));
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((7..8).into()))));
        assert_eq!(next!(lexer), Scalar(Bool(true)));
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((16..17).into()))));
        assert_eq!(next!(lexer), CompositeBegin(Array));
        assert_eq!(next!(lexer), Scalar(String(Plain((21..22).into()))));
        assert_eq!(next!(lexer), Scalar(Number((24..26).into())));
        assert_eq!(next!(lexer), Scalar(Null));
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), EntryEnd);
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_two_trivial_objects() {
        let input = br#"{"a":{"b":true}}{}"#;
        let mut lexer = Lexer::from_slice(input);
        assert_eq!(next!(lexer), EntryBegin);
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((2..3).into()))));
        assert_eq!(next!(lexer), CompositeBegin(Object));
        assert_eq!(next!(lexer), CompositeBegin(Field(Plain((7..8).into()))));
        assert_eq!(next!(lexer), Scalar(Bool(true)));
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), CompositeEnd);
        assert_eq!(next!(lexer), EntryEnd);
        assert_eq!(next!(lexer), EntryBegin);
        assert_eq!(next!(lexer), EntryEnd);
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn print_tokens() {
        let input = br#"{"@timestamp":"2021-06-20T00:00:00.393Z","@version":"1","agent":{"ephemeral_id":"30ca3b53-1ef6-4699-8728-7754d1698a01","hostname":"as-rtrf-fileboat-ajjke","id":"1a9b51ef-ffbe-420e-a92c-4f653afff5aa","type":"fileboat","version":"7.8.3"},"koent-id":"1280e812-654f-4d04-a4f8-e6b84079920a","anchor":"oglsaash","caller":"example/demo.go:200","dc_name":"as-rtrf","ecs":{"version":"1.0.0"},"host":{"name":"as-rtrf-fileboat-ajjke"},"input":{"type":"docker"},"kubernetes":{"container":{"name":"some-segway"},"labels":{"app":"some-segway","component":"some-segway","pod-template-hash":"756d998476","release":"as-rtrf-some-segway","subcomponent":"some-segway"},"namespace":"as-rtrf","node":{"name":"as-rtrf-k8s-kube-node-vm01"},"pod":{"name":"as-rtrf-some-segway-platform-756d998476-jz4jm","uid":"9d445b65-fbf7-4d94-a7f4-4dbb7753d65c"},"replicaset":{"name":"as-rtrf-some-segway-platform-756d998476"}},"level":"info","localTime":"2021-06-19T23:59:58.450Z","log":{"file":{"path":"/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log"},"offset":34009140},"logger":"deep","msg":"io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1","server-uuid":"0a1bec7f-a252-4ff6-994a-1fbdca318d6d","slot":2,"stream":"stdout","task-id":"1a632cba-8480-4644-93f2-262bc0c13d04","tenant-id":"40ddb7cf-ce50-41e4-b994-408e393355c0","time":"2021-06-20T00:00:00.393Z","ts":"2021-06-19T23:59:58.449489225Z","type":"k8s_containers_logs","unit":"0"}"#;
        let lexer = Lexer::from_slice(input);
        for token in lexer {
            println!("{:?}", token.unwrap());
        }
    }
}
