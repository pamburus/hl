// std imports
use std::str;

// local imports
use super::*;

// ---

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct JsonEncodedString<'a>(&'a str);

impl<'a> JsonEncodedString<'a> {
    #[inline]
    pub fn new(value: &'a str) -> Self {
        if value.len() < 2 || value.as_bytes()[0] != b'"' || value.as_bytes()[value.len() - 1] != b'"' {
            panic!("invalid JSON encoded string");
        }

        Self(value)
    }

    /// Decodes the string like [`AnyEncodedString::decode`], but replaces invalid escape
    /// sequences, such as lone surrogates, with U+FFFD instead of failing on them.
    #[inline]
    pub fn decode_lossy<H: Handler>(&self, handler: H) -> Result<()> {
        Parser::new(self.0).lossy().parse(handler)
    }
}

impl<'a> AnyEncodedString<'a> for JsonEncodedString<'a> {
    type Tokens = Tokens<'a>;

    #[inline]
    fn tokens(&self) -> Self::Tokens {
        Tokens::new(self.0)
    }

    #[inline]
    fn decode<H: Handler>(&self, handler: H) -> Result<()> {
        Parser::new(self.0).parse(handler)
    }

    #[inline]
    fn source(&self) -> &'a str {
        self.0
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.0 == r#""""#
    }
}

impl<'a> From<&'a str> for JsonEncodedString<'a> {
    #[inline]
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

// ---

pub struct Appender<'a> {
    buffer: &'a mut Vec<u8>,
}

impl<'a> Appender<'a> {
    #[inline]
    pub fn new(buffer: &'a mut Vec<u8>) -> Self {
        Self { buffer }
    }

    #[inline]
    fn handle_escape(&mut self, ch: u8) {
        self.buffer.push(b'\\');
        match ch {
            b'\x08' => self.buffer.push(b'b'),
            b'\x0c' => self.buffer.push(b'f'),
            b'\n' => self.buffer.push(b'n'),
            b'\r' => self.buffer.push(b'r'),
            b'\t' => self.buffer.push(b't'),
            b'\\' | b'"' => self.buffer.push(ch),
            _ => {
                self.buffer.extend(b"u00");
                self.buffer.push(HEX[((ch & 0xf0) >> 4) as usize]);
                self.buffer.push(HEX[(ch & 0x0f) as usize]);
            }
        }
    }
}

impl<'a> Handler for Appender<'a> {
    #[inline]
    fn handle(&mut self, token: Token<'_>) -> Option<()> {
        match token {
            Token::Char(ch) => match ch {
                ..='\x7f' => {
                    let ch = ch as u8;
                    if !ESCAPE[ch as usize] {
                        self.buffer.push(ch);
                    } else {
                        self.handle_escape(ch);
                    }
                }
                _ => {
                    let mut buf = [0; 4];
                    let s = ch.encode_utf8(&mut buf);
                    self.buffer.extend(s.as_bytes());
                }
            },
            Token::Sequence(s) => {
                let mut ss = s.as_bytes();
                while let Some(pos) = find_escape(ss) {
                    self.buffer.extend(&ss[..pos]);
                    self.handle_escape(ss[pos]);
                    ss = &ss[pos + 1..];
                }
                self.buffer.extend(ss);
            }
        }
        Some(())
    }
}

// ---

struct Parser<'a> {
    input: &'a str,
    index: usize,
    lossy: bool,
}

impl<'a> Parser<'a> {
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            index: 0,
            lossy: false,
        }
    }

    /// Makes the parser replace invalid escape sequences with U+FFFD.
    #[inline]
    fn lossy(self) -> Self {
        Self { lossy: true, ..self }
    }

    fn parse<H: Handler>(&mut self, mut handler: H) -> Result<()> {
        let extend =
            |handler: &mut H, s: &[u8]| handler.handle(Token::Sequence(unsafe { str::from_utf8_unchecked(s) }));

        self.next();
        let mut start = self.index;

        loop {
            let tail = &self.input()[self.index..];

            let pos = memchr::memchr2(b'"', b'\\', tail).unwrap_or(tail.len());

            self.index += pos;
            if self.index == self.input().len() {
                return Err(Error::Eof);
            }

            match tail[pos] {
                b'"' => {
                    extend(&mut handler, &tail[..pos]);
                    self.index += 1;

                    return Ok(());
                }
                b'\\' => {
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

    #[inline]
    fn input(&self) -> &'a [u8] {
        self.input.as_bytes()
    }

    #[inline]
    fn parse_escape(&mut self) -> Result<char> {
        if !self.lossy {
            return self.parse_escape_strict();
        }

        // A high surrogate that is not followed by a valid low surrogate is replaced on its own,
        // so whatever follows it is decoded as usual.
        let begin = self.index;
        match self.parse_escape_strict() {
            Ok(ch) => Ok(ch),
            Err(Error::Eof) => Err(Error::Eof),
            Err(Error::UnexpectedEndOfHexEscape | Error::LoneLeadingSurrogateInHexEscape)
                if self.input()[begin] == b'u' && (0xD800..=0xDBFF).contains(&self.hex_at(begin + 1)) =>
            {
                self.index = begin + 5;
                Ok(char::REPLACEMENT_CHARACTER)
            }
            Err(_) => Ok(char::REPLACEMENT_CHARACTER),
        }
    }

    /// Returns the value of the 4 hex digits at `index`, or 0 if they are not hex digits.
    #[inline]
    fn hex_at(&self, index: usize) -> u16 {
        let digits = self.input().get(index..index + 4).unwrap_or_default();
        digits
            .iter()
            .try_fold(0, |n, &b| decode_hex_val(b).map(|v| (n << 4) + v))
            .filter(|_| digits.len() == 4)
            .unwrap_or(0)
    }

    #[inline]
    fn parse_escape_strict(&mut self) -> Result<char> {
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

                        if !(0xDC00..=0xDFFF).contains(&n2) {
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
        for (i, &byte) in input.iter().enumerate().take(4) {
            let ch = decode_hex_val(byte);
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

    #[inline]
    fn peek(&mut self) -> Option<u8> {
        let input = self.input();
        if self.index < input.len() {
            Some(input[self.index])
        } else {
            None
        }
    }

    #[inline]
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

    #[inline]
    fn empty(&self) -> bool {
        self.index >= self.input().len()
    }
}

// ---

pub struct Tokens<'a>(Parser<'a>);

impl<'a> Tokens<'a> {
    #[inline]
    pub fn new(input: &'a str) -> Self {
        let mut parser = Parser::new(input);
        parser.next();
        Self(parser)
    }

    #[inline]
    fn input(&self) -> &'a [u8] {
        self.0.input.as_bytes()
    }

    #[inline]
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
        let tail = &self.0.input()[start..];
        let pos = memchr::memchr2(b'"', b'\\', tail).unwrap_or(tail.len());
        self.0.index += pos;
        let borrowed = &tail[..pos];
        Some(Ok(Token::Sequence(unsafe { str::from_utf8_unchecked(borrowed) })))
    }
}

// ---

/// Returns the position of the first byte in `bytes` that must be escaped.
#[inline]
fn find_escape(bytes: &[u8]) -> Option<usize> {
    const WORD: usize = size_of::<u64>();

    let mut words = bytes.chunks_exact(WORD);
    let mut offset = 0;
    for word in &mut words {
        let mask = escape_mask(u64::from_le_bytes(word.try_into().unwrap()));
        if mask != 0 {
            return Some(offset + (mask.trailing_zeros() / 8) as usize);
        }
        offset += WORD;
    }

    words
        .remainder()
        .iter()
        .position(|&b| ESCAPE[b as usize])
        .map(|pos| offset + pos)
}

/// Returns a mask with the high bit set in each byte of `word` that must be escaped.
/// Bytes above the lowest flagged byte may be flagged falsely, so only the lowest one is exact.
#[inline]
fn escape_mask(word: u64) -> u64 {
    const LO: u64 = u64::from_ne_bytes([0x01; 8]);
    const HI: u64 = u64::from_ne_bytes([0x80; 8]);

    #[inline]
    fn zeros(x: u64) -> u64 {
        x.wrapping_sub(LO) & !x & HI
    }

    let control = word.wrapping_sub(LO * 0x20) & !word & HI;
    let quote = zeros(word ^ (LO * b'"' as u64));
    let backslash = zeros(word ^ (LO * b'\\' as u64));
    let delete = zeros(word ^ (LO * 0x7f));

    control | quote | backslash | delete
}

// ---

#[inline]
fn decode_hex_val(val: u8) -> Option<u16> {
    let n = UNHEX[val as usize] as u16;
    if n == 255 { None } else { Some(n) }
}

// ---

// Lookup table of bytes that must be escaped. A value of true at index i means
// that byte i requires an escape sequence in the input.
static ESCAPE: [bool; 256] = {
    const CT: bool = true; // control character \x00..=\x1F
    const QU: bool = true; // quote \x22
    const BS: bool = true; // backslash \x5C
    const DL: bool = true; // DEL \x7F
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
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, DL, // 7
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

static HEX: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

static UNHEX: [u8; 256] = {
    const __: u8 = 255; // not a hex digit
    #[allow(clippy::zero_prefixed_literal)]
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
mod tests;
