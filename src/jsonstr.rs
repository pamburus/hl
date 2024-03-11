use std::str;

use crate::error::{Error, Result};

pub trait Handler {
    fn push(&mut self, b: u8);
    fn extend(&mut self, s: &str);
}

struct Parser<'a> {
    input: &'a [u8],
    index: usize,
}

impl Parser<'_> {
    fn parse<H: Handler>(&mut self, handler: &mut H) -> Result<()> {
        self.next();
        let mut no_escapes = true;
        let mut start = self.index;

        loop {
            while self.index < self.input.len() && !ESCAPE[self.input[self.index] as usize] {
                self.index += 1;
            }
            if self.index == self.input.len() {
                return Err(Error::JsonParseError(()));
            }
            match self.input[self.index] {
                b'"' => {
                    if no_escapes {
                        let borrowed = &self.input[start..self.index];
                        self.index += 1;
                        handler.extend(unsafe { str::from_utf8_unchecked(borrowed) });
                        return Ok(());
                    }

                    handler.extend(unsafe { str::from_utf8_unchecked(&self.input[start..self.index]) });
                    self.index += 1;

                    return Ok(());
                }
                b'\\' => {
                    no_escapes = false;
                    handler.extend(unsafe { str::from_utf8_unchecked(&self.input[start..self.index]) });
                    self.index += 1;
                    self.parse_escape(handler)?;
                    start = self.index;
                }
                _ => {
                    self.index += 1;
                    return Err(Error::UnexpectedControlCharacter);
                }
            }
        }
    }

    fn parse_escape<H: Handler>(&mut self, handler: &mut H) -> Result<()> {
        let Some(ch) = self.next() else {
            return Err(Error::Eof);
        };

        match ch {
            b'"' => handler.push(b'"'),
            b'\\' => handler.push(b'\\'),
            b'/' => handler.push(b'/'),
            b'b' => handler.push(b'\x08'),
            b'f' => handler.push(b'\x0c'),
            b'n' => handler.push(b'\n'),
            b'r' => handler.push(b'\r'),
            b't' => handler.push(b'\t'),
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

                handler.extend(c.encode_utf8(&mut [0_u8; 4]));
            }
            _ => {
                return Err(Error::InvalidEscape);
            }
        }

        Ok(())
    }

    fn decode_hex_escape(&mut self) -> Result<u16> {
        if self.input.len() < 4 {
            self.input = &self.input[self.input.len()..];
            return Err(Error::Eof);
        }

        let mut n = 0;
        for i in 0..4 {
            let ch = decode_hex_val(self.input[i]);
            match ch {
                None => {
                    self.input = &self.input[i..];
                    return Err(Error::InvalidEscape);
                }
                Some(val) => {
                    n = (n << 4) + val;
                }
            }
        }
        self.input = &self.input[4..];
        Ok(n)
    }

    #[inline]
    fn peek(&mut self) -> Option<u8> {
        if self.index < self.input.len() {
            Some(self.input[self.index])
        } else {
            None
        }
    }

    #[inline]
    fn next(&mut self) -> Option<u8> {
        if self.index < self.input.len() {
            let ch = self.input[self.index];
            self.index += 1;
            Some(ch)
        } else {
            None
        }
    }
}

#[inline]
fn decode_hex_val(val: u8) -> Option<u16> {
    let n = HEX[val as usize] as u16;
    if n == 255 {
        None
    } else {
        Some(n)
    }
}

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
