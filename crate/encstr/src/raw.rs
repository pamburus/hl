// local imports
use super::*;

// ---

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct RawString<'a>(&'a str);

impl<'a> RawString<'a> {
    #[inline(always)]
    pub fn new(value: &'a str) -> Self {
        Self(value)
    }

    #[inline(always)]
    pub fn as_str(&self) -> &'a str {
        self.0
    }
}

impl<'a> std::ops::Deref for RawString<'a> {
    type Target = &'a str;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> AnyEncodedString<'a> for RawString<'a> {
    type Tokens = Tokens<'a>;

    #[inline(always)]
    fn decode<H: Handler>(&self, mut handler: H) -> Result<()> {
        handler.handle(Token::Sequence(self.0));
        Ok(())
    }

    #[inline(always)]
    fn tokens(&self) -> Self::Tokens {
        Tokens(Some(self.0))
    }

    #[inline(always)]
    fn source(&self) -> &'a str {
        self.0
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> From<&'a str> for RawString<'a> {
    #[inline(always)]
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

// ---

pub struct Tokens<'a>(Option<&'a str>);

impl<'a> Iterator for Tokens<'a> {
    type Item = Result<Token<'a>>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().map(|s| Ok(Token::Sequence(s)))
    }
}

// ---

pub struct Appender<'a> {
    buffer: &'a mut Vec<u8>,
}

impl<'a> Appender<'a> {
    #[inline(always)]
    pub fn new(buffer: &'a mut Vec<u8>) -> Self {
        Self { buffer }
    }
}

impl<'a> Handler for Appender<'a> {
    #[inline(always)]
    fn handle(&mut self, token: Token<'_>) -> Option<()> {
        match token {
            Token::Char(ch) => match ch {
                ..='\x7f' => self.buffer.push(ch as u8),
                _ => {
                    let mut buf = [0; 4];
                    let s = ch.encode_utf8(&mut buf);
                    self.buffer.extend(s.as_bytes());
                }
            },
            Token::Sequence(s) => self.buffer.extend(s.as_bytes()),
        }
        Some(())
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_string() {
        let mut result = Builder::new();
        let string = RawString::new("hello, world!¡");
        string.decode(&mut result).unwrap();
        assert_eq!(result.as_str(), "hello, world!¡");
    }

    #[test]
    fn test_appender() {
        let mut buffer = Vec::new();
        let mut appender = Appender::new(&mut buffer);
        appender.handle(Token::Sequence("hello ")).unwrap();
        appender.handle(Token::Char('•')).unwrap();
        appender.handle(Token::Sequence(" world")).unwrap();
        assert_eq!(std::str::from_utf8(&buffer).unwrap(), "hello • world");
    }
}
