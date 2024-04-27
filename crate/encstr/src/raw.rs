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
        handler.handle(Token::Sequence(self.0.as_ref()));
        Ok(())
    }

    #[inline(always)]
    fn tokens(&self) -> Self::Tokens {
        Tokens(Some(self.0.as_ref()))
    }

    #[inline(always)]
    fn source(&self) -> &'a str {
        self.0.as_ref()
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
        self.0.take().map(|s| Ok(Token::Sequence(s.as_ref())))
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_string() {
        let mut result = Builder::new();
        let string = RawString::new("hello, world!");
        string.decode(&mut result).unwrap();
        assert_eq!(result.as_str(), "hello, world!");
    }
}
