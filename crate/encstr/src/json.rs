// std imports
use std::str;

// local imports
use super::*;

// ---

#[derive(Clone, Copy)]
pub struct JsonEncodedString<'a>(&'a str);

impl<'a> JsonEncodedString<'a> {
    #[inline(always)]
    pub fn new(value: &'a str) -> Self {
        Self(value)
    }
}

impl<'a> AnyEncodedString<'a> for JsonEncodedString<'a> {
    type Tokens = Tokens<'a>;

    #[inline(always)]
    fn tokens(&self) -> Self::Tokens {
        Tokens::new(self.0.as_ref())
    }

    #[inline(always)]
    fn decode<H: Handler>(&self, handler: H) -> Result<()> {
        Parser::new(self.0.as_ref()).parse(handler)
    }

    #[inline(always)]
    fn source(&self) -> &'a str {
        self.0
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.0 == r#""""#
    }
}

impl<'a> From<&'a str> for JsonEncodedString<'a> {
    #[inline(always)]
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

// ---

struct Parser<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> Parser<'a> {
    #[inline(always)]
    pub fn new(input: &'a str) -> Self {
        Self { input, index: 0 }
    }

    fn parse<H: Handler>(&mut self, mut handler: H) -> Result<()> {
        let extend =
            |handler: &mut H, s: &[u8]| handler.handle(Token::Sequence(unsafe { str::from_utf8_unchecked(s) }));

        self.next();
        let mut no_escapes = true;
        let mut start = self.index;

        loop {
            while self.peek().map(|ch| ESCAPE[usize::from(ch)]) == Some(false) {
                self.index += 1;
            }
            if self.index == self.input().len() {
                return Err(Error::Eof);
            }
            match self.input()[self.index] {
                b'"' => {
                    if no_escapes {
                        let borrowed = &self.input()[start..self.index];
                        extend(&mut handler, borrowed);
                        self.index += 1;
                        return Ok(());
                    }

                    extend(&mut handler, &self.input()[start..self.index]);
                    self.index += 1;

                    return Ok(());
                }
                b'\\' => {
                    no_escapes = false;
                    extend(&mut handler, &self.input()[start..self.index]);
                    self.index += 1;
                    handler.handle(Token::Char(self.parse_escape()?));
                    start = self.index;
                }
                _ => {
                    self.index += 1;
                    return Err(Error::UnexpectedControlCharacter);
                }
            }
        }
    }

    #[inline(always)]
    fn input(&self) -> &'a [u8] {
        self.input.as_bytes()
    }

    fn parse_escape(&mut self) -> Result<char> {
        let Some(ch) = self.next() else {
            return Err(Error::Eof);
        };

        match ch {
            b'"' => Ok('"'),
            b'\\' => Ok('\\'),
            b'/' => Ok('/'),
            b'b' => Ok('\x08'),
            b'f' => Ok('\x0c'),
            b'n' => Ok('\n'),
            b'r' => Ok('\r'),
            b't' => Ok('\t'),
            b'u' => {
                let c = match self.decode_hex_escape()? {
                    0xDC00..=0xDFFF => {
                        return Err(Error::LoneLeadingSurrogateInHexEscape);
                    }

                    n1 @ 0xD800..=0xDBFF => {
                        if self.peek() == Some(b'\\') {
                            self.next();
                        } else {
                            return Err(Error::UnexpectedEndOfHexEscape);
                        }

                        if self.peek() == Some(b'u') {
                            self.next();
                        } else {
                            return Err(Error::UnexpectedEndOfHexEscape);
                        }

                        let n2 = self.decode_hex_escape()?;

                        if n2 < 0xDC00 || n2 > 0xDFFF {
                            return Err(Error::LoneLeadingSurrogateInHexEscape);
                        }

                        let n = (((n1 - 0xD800) as u32) << 10 | (n2 - 0xDC00) as u32) + 0x1_0000;

                        match char::from_u32(n) {
                            Some(c) => c,
                            None => {
                                return Err(Error::InvalidUnicodeCodePoint);
                            }
                        }
                    }

                    n => char::from_u32(n as u32).unwrap(),
                };

                Ok(c)
            }
            _ => Err(Error::InvalidEscape),
        }
    }

    #[inline]
    fn decode_hex_escape(&mut self) -> Result<u16> {
        let input = &self.input()[self.index..];

        if input.len() < 4 {
            self.index = input.len();
            return Err(Error::Eof);
        }

        let mut n = 0;
        for i in 0..4 {
            let ch = decode_hex_val(input[i]);
            match ch {
                None => {
                    self.index += i;
                    return Err(Error::InvalidEscape);
                }
                Some(val) => {
                    n = (n << 4) + val;
                }
            }
        }
        self.index += 4;
        Ok(n)
    }

    #[inline(always)]
    fn peek(&mut self) -> Option<u8> {
        let input = self.input();
        if self.index < input.len() {
            Some(input[self.index])
        } else {
            None
        }
    }

    #[inline(always)]
    fn next(&mut self) -> Option<u8> {
        let input = self.input();
        if self.index < input.len() {
            let ch = input[self.index];
            self.index += 1;
            Some(ch)
        } else {
            None
        }
    }

    #[inline(always)]
    fn empty(&self) -> bool {
        self.index >= self.input().len()
    }
}

// ---

pub struct Tokens<'a>(Parser<'a>);

impl<'a> Tokens<'a> {
    #[inline(always)]
    pub fn new(input: &'a str) -> Self {
        let mut parser = Parser::new(input);
        parser.next();
        Self(parser)
    }

    #[inline(always)]
    fn input(&self) -> &'a [u8] {
        self.0.input.as_bytes()
    }

    #[inline(always)]
    fn peek(&self) -> u8 {
        self.input()[self.0.index]
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Result<Token<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.empty() {
            return None;
        }

        let head = self.peek();
        if ESCAPE[head as usize] {
            return match head {
                b'"' => {
                    self.0.index += 1;
                    None
                }
                b'\\' => {
                    let begin = self.0.index;
                    self.0.index += 1;
                    let result = self.0.parse_escape().map(Token::Char);
                    if result.is_err() {
                        self.0.index = begin;
                    }
                    Some(result)
                }
                _ => Some(Err(Error::UnexpectedControlCharacter)),
            };
        }

        let start = self.0.index;
        let n = self.0.input()[start..]
            .iter()
            .position(|&ch| ESCAPE[ch as usize])
            .unwrap_or(self.0.input().len() - start);
        self.0.index += n;
        let borrowed = &self.input()[start..self.0.index];
        Some(Ok(Token::Sequence(unsafe { str::from_utf8_unchecked(borrowed) })))
    }
}

// ---

#[inline(always)]
fn decode_hex_val(val: u8) -> Option<u16> {
    let n = HEX[val as usize] as u16;
    if n == 255 {
        None
    } else {
        Some(n)
    }
}

// ---

// Lookup table of bytes that must be escaped. A value of true at index i means
// that byte i requires an escape sequence in the input.
static ESCAPE: [bool; 256] = {
    const CT: bool = true; // control character \x00..=\x1F
    const QU: bool = true; // quote \x22
    const BS: bool = true; // backslash \x5C
    const __: bool = false; // allow unescaped
    [
        //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
        CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 0
        CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 1
        __, __, QU, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 3
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
        __, __, __, __, __, __, __, __, __, __, __, __, BS, __, __, __, // 5
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
    ]
};

static HEX: [u8; 256] = {
    const __: u8 = 255; // not a hex digit
    [
        //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 0
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 1
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
        00, 01, 02, 03, 04, 05, 06, 07, 08, 09, __, __, __, __, __, __, // 3
        __, 10, 11, 12, 13, 14, 15, __, __, __, __, __, __, __, __, __, // 4
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 5
        __, 10, 11, 12, 13, 14, 15, __, __, __, __, __, __, __, __, __, // 6
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
    ]
};

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_hex_val() {
        assert_eq!(decode_hex_val(b'0'), Some(0));
        assert_eq!(decode_hex_val(b'1'), Some(1));
        assert_eq!(decode_hex_val(b'2'), Some(2));
        assert_eq!(decode_hex_val(b'3'), Some(3));
        assert_eq!(decode_hex_val(b'4'), Some(4));
        assert_eq!(decode_hex_val(b'5'), Some(5));
        assert_eq!(decode_hex_val(b'6'), Some(6));
        assert_eq!(decode_hex_val(b'7'), Some(7));
        assert_eq!(decode_hex_val(b'8'), Some(8));
        assert_eq!(decode_hex_val(b'9'), Some(9));
        assert_eq!(decode_hex_val(b'A'), Some(10));
        assert_eq!(decode_hex_val(b'B'), Some(11));
        assert_eq!(decode_hex_val(b'C'), Some(12));
        assert_eq!(decode_hex_val(b'D'), Some(13));
        assert_eq!(decode_hex_val(b'E'), Some(14));
        assert_eq!(decode_hex_val(b'F'), Some(15));
        assert_eq!(decode_hex_val(b'G'), None);
        assert_eq!(decode_hex_val(b'g'), None);
        assert_eq!(decode_hex_val(b' '), None);
        assert_eq!(decode_hex_val(b'\n'), None);
        assert_eq!(decode_hex_val(b'\r'), None);
        assert_eq!(decode_hex_val(b'\t'), None);
    }

    #[test]
    fn test_parser() {
        let mut result = Builder::new();
        let mut parser = Parser::new(r#""hello, \"world\"""#);
        parser.parse(&mut result).unwrap();
        assert_eq!(result.as_str(), "hello, \"world\"");
    }

    #[test]
    fn test_tokens() {
        let mut tokens = Tokens::new(&r#""hello, \"world\"""#);
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
        assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("world"))));
        assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
        assert_eq!(tokens.next(), None);
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_tokens_escape() {
        let mut tokens = Tokens::new(&r#""hello, \\\"world\"""#);
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
        assert_eq!(tokens.next(), Some(Ok(Token::Char('\\'))));
        assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("world"))));
        assert_eq!(tokens.next(), Some(Ok(Token::Char('"'))));
        assert_eq!(tokens.next(), None);
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_tokens_control() {
        let mut tokens = Tokens::new(&r#""hello, \x00world""#);
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
        assert_eq!(tokens.next(), Some(Err(Error::InvalidEscape)));
        assert_eq!(tokens.next(), Some(Err(Error::InvalidEscape)));
    }

    #[test]
    fn test_tokens_eof() {
        let mut tokens = Tokens::new(&r#""hello, \u"#);
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
        assert_eq!(tokens.next(), Some(Err(Error::Eof)));
        assert_eq!(tokens.next(), Some(Err(Error::Eof)));
    }

    #[test]
    fn test_tokens_lone_surrogate() {
        let mut tokens = Tokens::new(&r#""hello, \udc00world""#);
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
        assert_eq!(tokens.next(), Some(Err(Error::LoneLeadingSurrogateInHexEscape)));
    }

    #[test]
    fn test_tokens_unexpected_end() {
        let mut tokens = Tokens::new(&r#""hello, \ud800""#);
        assert_eq!(tokens.next(), Some(Ok(Token::Sequence("hello, "))));
        assert_eq!(tokens.next(), Some(Err(Error::UnexpectedEndOfHexEscape)));
    }
}
