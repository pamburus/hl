use std::str;

use super::*;

// ---

pub trait AnyEncodedString<'a> {
    type Tokens: Iterator<Item = Result<Token<'a>>>;

    fn decode<H: Handler>(&self, handler: H) -> Result<()>;
    fn tokens(&self) -> Self::Tokens;
    fn source(&self) -> &'a str;
    fn is_empty(&self) -> bool;

    #[inline(always)]
    fn chars(&self) -> Chars<'a, Self::Tokens> {
        Chars::new(self.tokens())
    }

    #[inline(always)]
    fn bytes(&self) -> Bytes<'a, Self::Tokens> {
        Bytes::new(self.tokens())
    }
}

// ---

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum EncodedString<'a> {
    Json(super::json::JsonEncodedString<'a>),
    Raw(super::raw::RawString<'a>),
}

impl<'a> EncodedString<'a> {
    #[inline(always)]
    pub fn json(value: &'a str) -> Self {
        EncodedString::Json(super::json::JsonEncodedString::new(value))
    }

    #[inline(always)]
    pub fn raw(value: &'a str) -> Self {
        EncodedString::Raw(super::raw::RawString::new(value))
    }

    #[inline(always)]
    pub fn source(&self) -> &'a str {
        match self {
            EncodedString::Json(string) => string.source(),
            EncodedString::Raw(string) => string.source(),
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            EncodedString::Json(string) => string.is_empty(),
            EncodedString::Raw(string) => string.is_empty(),
        }
    }
}

impl<'a> AnyEncodedString<'a> for EncodedString<'a> {
    type Tokens = EncodedStringTokens<'a>;

    #[inline(always)]
    fn decode<H: Handler>(&self, handler: H) -> Result<()> {
        match self {
            EncodedString::Json(string) => string.decode(handler),
            EncodedString::Raw(string) => string.decode(handler),
        }
    }

    #[inline(always)]
    fn tokens(&self) -> Self::Tokens {
        match self {
            EncodedString::Json(string) => Self::Tokens::Json(string.tokens()),
            EncodedString::Raw(string) => Self::Tokens::Raw(string.tokens()),
        }
    }

    #[inline(always)]
    fn source(&self) -> &'a str {
        self.source()
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        match self {
            EncodedString::Json(string) => string.is_empty(),
            EncodedString::Raw(string) => string.is_empty(),
        }
    }
}

pub enum EncodedStringTokens<'a> {
    Json(super::json::Tokens<'a>),
    Raw(super::raw::Tokens<'a>),
}

impl<'a> Iterator for EncodedStringTokens<'a> {
    type Item = Result<Token<'a>>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EncodedStringTokens::Json(tokens) => tokens.next(),
            EncodedStringTokens::Raw(tokens) => tokens.next(),
        }
    }
}

// ---

pub struct Chars<'a, T> {
    tokens: T,
    s: str::Chars<'a>,
}

impl<'a, T> Chars<'a, T>
where
    T: Iterator<Item = Result<Token<'a>>>,
{
    #[inline(always)]
    pub fn new(tokens: T) -> Self {
        Self { tokens, s: "".chars() }
    }
}

impl<'a, T> Iterator for Chars<'a, T>
where
    T: Iterator<Item = Result<Token<'a>>>,
{
    type Item = Result<char>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ch) = self.s.next() {
                return Some(Ok(ch));
            }

            match self.tokens.next() {
                Some(Ok(Token::Char(ch))) => return Some(Ok(ch)),
                Some(Ok(Token::Sequence(s))) => self.s = s.chars(),
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            }
        }
    }
}

// ---

pub struct Bytes<'a, T> {
    head: Option<BytesHead<'a>>,
    tail: T,
}

impl<'a, T> Bytes<'a, T>
where
    T: Iterator<Item = Result<Token<'a>>>,
{
    #[inline(always)]
    pub fn new(tokens: T) -> Self {
        Self {
            head: None,
            tail: tokens,
        }
    }
}

impl<'a, T> Iterator for Bytes<'a, T>
where
    T: Iterator<Item = Result<Token<'a>>>,
{
    type Item = Result<u8>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.head {
                Some(BytesHead::Char(s, n, ref mut i)) => {
                    if *i < n {
                        let j = *i;
                        *i += 1;
                        return Some(Ok(s[j]));
                    }
                }
                Some(BytesHead::Sequence(ref mut s)) => {
                    if let Some(ch) = s.next() {
                        return Some(Ok(ch));
                    }
                }
                None => {}
            }

            match self.tail.next() {
                Some(Ok(Token::Char(ch @ '\x00'..='\x7F'))) => return Some(Ok(ch as u8)),
                Some(Ok(Token::Char(ch))) => {
                    let mut buf: [u8; 4] = [0; 4];
                    let n = ch.encode_utf8(&mut buf).len();
                    self.head = Some(BytesHead::Char(buf, n, 0));
                }
                Some(Ok(Token::Sequence(s))) => self.head = Some(BytesHead::Sequence(s.bytes())),
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            }
        }
    }
}

enum BytesHead<'a> {
    Char([u8; 4], usize, usize),
    Sequence(str::Bytes<'a>),
}

// ---

pub trait Handler {
    fn handle(&mut self, token: Token<'_>) -> Option<()>;
}

impl<H> Handler for &mut H
where
    H: Handler,
{
    #[inline(always)]
    fn handle(&mut self, token: Token<'_>) -> Option<()> {
        (**self).handle(token)
    }
}

impl Handler for &mut Vec<u8> {
    #[inline(always)]
    fn handle(&mut self, token: Token<'_>) -> Option<()> {
        RawAppender::new(self).handle(token)
    }
}

// ---

pub struct HandlerFn<F>(F);

impl<F> HandlerFn<F> {
    #[inline(always)]
    pub fn new(f: F) -> Self {
        HandlerFn(f)
    }
}

impl<F> Handler for HandlerFn<F>
where
    F: FnMut(Token<'_>) -> Option<()>,
{
    #[inline(always)]
    fn handle(&mut self, token: Token<'_>) -> Option<()> {
        self.0(token)
    }
}

// ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token<'a> {
    Char(char),
    Sequence(&'a str),
}

// ---

pub struct Builder {
    buffer: Vec<u8>,
}

impl Builder {
    #[inline(always)]
    pub fn new() -> Self {
        Builder { buffer: Vec::new() }
    }

    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Builder {
            buffer: Vec::with_capacity(capacity),
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    #[inline(always)]
    pub fn into_string(self) -> String {
        unsafe { String::from_utf8_unchecked(self.buffer) }
    }

    #[inline(always)]
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.buffer) }
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }
}

impl Handler for Builder {
    #[inline(always)]
    fn handle(&mut self, token: Token<'_>) -> Option<()> {
        RawAppender::new(&mut self.buffer).handle(token)
    }
}

// ---

pub struct Ignorer;

impl Handler for Ignorer {
    #[inline(always)]
    fn handle(&mut self, _: Token<'_>) -> Option<()> {
        None
    }
}

// ---

pub trait AsBytes {
    fn as_bytes(&self) -> &[u8];
}

impl AsBytes for &str {
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        (*self).as_bytes()
    }
}

impl AsBytes for String {
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsBytes for &[u8] {
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl AsBytes for Vec<u8> {
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<S> AsBytes for &S
where
    S: AsBytes,
{
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        (*self).as_bytes()
    }
}

// ---

pub trait Text<'a> {
    type Out: 'a + AsRef<str> + From<&'a str> + PartialEq + Eq + Clone;
}

impl<'a> Text<'a> for String {
    type Out = Self;
}

impl<'a> Text<'a> for &'a str {
    type Out = Self;
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder() {
        let mut result = Builder::new();
        result.handle(Token::Char('h')).unwrap();
        result.handle(Token::Char('e')).unwrap();
        result.handle(Token::Char('l')).unwrap();
        result.handle(Token::Char('l')).unwrap();
        result.handle(Token::Char('o')).unwrap();
        result.handle(Token::Char(',')).unwrap();
        result.handle(Token::Char(' ')).unwrap();
        result.handle(Token::Char('w')).unwrap();
        result.handle(Token::Char('o')).unwrap();
        result.handle(Token::Char('r')).unwrap();
        result.handle(Token::Char('l')).unwrap();
        result.handle(Token::Char('d')).unwrap();
        assert_eq!(result.as_str(), "hello, world");
    }

    #[test]
    fn builder_default() {
        let builder1 = Builder::new();
        let builder2 = Builder::default();

        // Both should start with empty content
        assert_eq!(builder1.as_str(), "");
        assert_eq!(builder2.as_str(), "");

        // Both should have the same capacity
        assert_eq!(builder1.buffer.capacity(), builder2.buffer.capacity());
    }

    #[test]
    fn encoded_string_raw() {
        let s = EncodedString::raw("hello, world!");
        let mut tokens = s.tokens();
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, world!"))));
        assert_eq!(tokens.next(), None);
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn encoded_string_json() {
        let s = EncodedString::json(r#""hello, \"world\"!""#);
        let mut tokens = s.tokens();
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
        assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("world"))));
        assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("!"))));
        assert_eq!(tokens.next(), None);
        assert_eq!(tokens.next(), None);
    }
}
