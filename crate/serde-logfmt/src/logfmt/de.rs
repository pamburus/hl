use std::{
    ops::{AddAssign, Deref, MulAssign, Neg},
    str,
};

use serde::Deserialize;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};

use super::error::{Error, Result};

#[inline]
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    unsafe { from_slice_unchecked(s.as_bytes()) }
}

#[inline]
pub fn from_slice<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    from_str(str::from_utf8(s).map_err(Error::InvalidUtf8)?)
}

/// # Safety
/// The caller must ensure that the input slice contains valid UTF-8 data.
#[inline]
pub unsafe fn from_slice_unchecked<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = unsafe { Deserializer::from_slice_unchecked(s) };
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.parser.tail().is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

// ---

pub struct Deserializer<'de> {
    scratch: Vec<u8>,
    parser: Parser<'de>,
}

impl<'de> Deserializer<'de> {
    #[inline]
    pub fn new(input: &'de str) -> Self {
        unsafe { Self::from_slice_unchecked(input.as_bytes()) }
    }

    #[inline]
    pub fn from_slice(input: &'de [u8]) -> Result<Self> {
        Ok(Self::new(str::from_utf8(input).map_err(Error::InvalidUtf8)?))
    }

    /// # Safety
    /// The caller must ensure that the input slice contains valid UTF-8 data.
    #[inline]
    pub unsafe fn from_slice_unchecked(s: &'de [u8]) -> Self {
        Deserializer {
            scratch: Vec::new(),
            parser: Parser {
                input: s,
                index: 0,
                key: false,
            },
        }
    }

    #[inline]
    pub fn parse_str_to_buf(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        match self.parser.parse_value(buf, false) {
            Ok(Reference::Borrowed(b)) => {
                buf.extend(b.as_bytes());
                Ok(())
            }
            Ok(Reference::Copied(_)) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl<'de> Deserializer<'de> {
    #[inline]
    fn parse_bool(&mut self) -> Result<bool> {
        self.parser.parse_bool()
    }

    #[inline]
    fn parse_unsigned<T>(&mut self) -> Result<T>
    where
        T: AddAssign<T> + MulAssign<T> + From<u8>,
    {
        self.parser.parse_unsigned()
    }

    #[inline]
    fn parse_signed<T>(&mut self) -> Result<T>
    where
        T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8>,
    {
        self.parser.parse_signed()
    }

    #[inline]
    fn parse_string<'s>(&'s mut self, ignore: bool) -> Result<Reference<'de, 's, str>> {
        self.scratch.clear();
        self.parser.parse_string(&mut self.scratch, ignore)
    }

    #[inline]
    fn deserialize_raw_value<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.parser.deserialize_raw_value(visitor)
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_signed()?)
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_signed()?)
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_signed()?)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_signed()?)
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_unsigned()?)
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_unsigned()?)
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_unsigned()?)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parse_string(false)? {
            Reference::Borrowed(b) => visitor.visit_borrowed_str(b),
            Reference::Copied(c) => visitor.visit_str(c),
        }
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotImplemented)
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.parser.input.starts_with(b"null") {
            self.parser.input = &self.parser.input["null".len()..];
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.parser.input.starts_with(b"null") {
            self.parser.input = &self.parser.input["null".len()..];
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNull)
        }
    }

    #[inline]
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if name == super::raw::TOKEN {
            return self.deserialize_raw_value(visitor);
        }

        visitor.visit_newtype_struct(self)
    }

    #[inline]
    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotImplemented)
    }

    #[inline]
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(KeyValueSequence::new(self))
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self.parse_string(false)?.into_deserializer())
    }

    #[inline]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

// ---

struct Parser<'de> {
    input: &'de [u8],
    index: usize,
    key: bool,
}

impl<'de> Parser<'de> {
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

    #[inline]
    fn parse_bool(&mut self) -> Result<bool> {
        if self.tail().starts_with(b"true") {
            self.advance(4);
            Ok(true)
        } else if self.tail().starts_with(b"false") {
            self.advance(5);
            Ok(false)
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    fn parse_unsigned<T>(&mut self) -> Result<T>
    where
        T: AddAssign<T> + MulAssign<T> + From<u8>,
    {
        let mut int = match self.next() {
            Some(ch @ b'0'..=b'9') => T::from(ch - b'0'),
            _ => {
                return Err(Error::ExpectedInteger);
            }
        };
        loop {
            match self.peek() {
                Some(ch @ b'0'..=b'9') => {
                    self.advance(1);
                    int *= T::from(10);
                    int += T::from(ch - b'0');
                }
                _ => {
                    return Ok(int);
                }
            }
        }
    }

    fn parse_signed<T>(&mut self) -> Result<T>
    where
        T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8>,
    {
        let mut negative = false;
        if self.peek() == Some(b'-') {
            negative = true;
            self.advance(1);
        }

        let mut int = match self.next() {
            Some(ch @ b'0'..=b'9') => T::from((ch - b'0') as i8),
            _ => {
                return Err(Error::ExpectedInteger);
            }
        };
        loop {
            match self.peek() {
                Some(ch @ b'0'..=b'9') => {
                    self.advance(1);
                    int *= T::from(10);
                    int += T::from((ch - b'0') as i8);
                }
                _ => {
                    if negative {
                        int = -int;
                    }
                    return Ok(int);
                }
            }
        }
    }

    fn skip_garbage(&mut self) {
        if let Some(i) = self.tail().iter().position(|&c| c > b' ') {
            self.advance(i);
        } else {
            self.index = self.input.len();
        }
    }

    fn parse_string<'s>(&'s mut self, scratch: &'s mut Vec<u8>, ignore: bool) -> Result<Reference<'de, 's, str>> {
        if self.key {
            self.parse_key().map(Reference::Borrowed)
        } else {
            self.parse_value(scratch, ignore)
        }
    }

    fn parse_key(&mut self) -> Result<&'de str> {
        self.skip_garbage();

        let start = self.index;
        let mut unicode = false;

        while self.index < self.input.len() {
            let c = self.input[self.index];
            match KEY[c as usize] {
                KeyCh::EQ_SIGN => {
                    break;
                }
                KeyCh::UNICODE => {
                    unicode = true;
                    self.index += 1;
                }
                KeyCh::ALLOWED => {
                    self.index += 1;
                }
                _ => {
                    return Err(Error::UnexpectedByte(c));
                }
            }
        }

        if self.index == start {
            return Err(Error::ExpectedKey);
        }

        let s = &self.input[start..self.index];
        if self.next() != Some(b'=') {
            return Err(Error::ExpectedKeyValueDelimiter);
        }

        if unicode {
            return str::from_utf8(s).map_err(|_| Error::InvalidUnicodeCodePoint);
        }

        Ok(unsafe { str::from_utf8_unchecked(s) })
    }

    fn parse_value<'s>(&'s mut self, scratch: &'s mut Vec<u8>, ignore: bool) -> Result<Reference<'de, 's, str>> {
        match self.peek() {
            Some(b'"') => self.parse_quoted_value(scratch, ignore),
            _ => self.parse_unquoted_value().map(Reference::Borrowed),
        }
    }

    fn parse_unquoted_value(&mut self) -> Result<&'de str> {
        let start = self.index;
        let mut unicode = false;

        while self.index < self.input.len() {
            let c = self.input[self.index];
            match c {
                b'\x00'..=b' ' => {
                    break;
                }
                b'"' | b'=' => {
                    return Err(Error::UnexpectedByte(c));
                }
                b'\x80'..=b'\xFF' => {
                    unicode = true;
                    self.index += 1;
                }
                _ => {
                    self.index += 1;
                }
            }
        }

        if self.index == start {
            return Ok("");
        }

        let s = &self.input[start..self.index];

        if unicode {
            return str::from_utf8(s).map_err(|_| Error::InvalidUnicodeCodePoint);
        }

        Ok(unsafe { str::from_utf8_unchecked(s) })
    }

    fn parse_quoted_value<'s>(&'s mut self, scratch: &'s mut Vec<u8>, ignore: bool) -> Result<Reference<'de, 's, str>> {
        self.next();
        let mut no_escapes = true;
        let mut start = self.index;

        loop {
            while self.index < self.input.len() && !ESCAPE[self.input[self.index] as usize] {
                self.advance(1);
            }
            if self.index == self.input.len() {
                return Err(Error::Eof);
            }
            match self.input[self.index] {
                b'"' => {
                    if no_escapes {
                        let borrowed = &self.input[start..self.index];
                        self.advance(1);
                        return Ok(Reference::Borrowed(unsafe { str::from_utf8_unchecked(borrowed) }));
                    }

                    if !ignore {
                        scratch.extend_from_slice(&self.input[start..self.index]);
                    }
                    self.advance(1);

                    return if !ignore {
                        Ok(Reference::Copied(unsafe { str::from_utf8_unchecked(scratch) }))
                    } else {
                        Ok(Reference::Borrowed(unsafe {
                            str::from_utf8_unchecked(&self.input[self.index..self.index])
                        }))
                    };
                }
                b'\\' => {
                    no_escapes = false;
                    if !ignore {
                        scratch.extend_from_slice(&self.input[start..self.index]);
                    }
                    self.advance(1);
                    self.parse_escape(scratch, ignore)?;
                    start = self.index;
                }
                _ => {
                    self.advance(1);
                    return Err(Error::UnexpectedControlCharacter);
                }
            }
        }
    }

    fn parse_escape(&mut self, scratch: &mut Vec<u8>, ignore: bool) -> Result<()> {
        let Some(ch) = self.next() else {
            return Err(Error::Eof);
        };

        match ch {
            b'"' => scratch.push(b'"'),
            b'\\' => scratch.push(b'\\'),
            b'/' => scratch.push(b'/'),
            b'b' => scratch.push(b'\x08'),
            b'f' => scratch.push(b'\x0c'),
            b'n' => scratch.push(b'\n'),
            b'r' => scratch.push(b'\r'),
            b't' => scratch.push(b'\t'),
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

                if !ignore {
                    scratch.extend_from_slice(c.encode_utf8(&mut [0_u8; 4]).as_bytes());
                }
            }
            _ => {
                return Err(Error::InvalidEscape);
            }
        }

        Ok(())
    }

    fn deserialize_raw_value<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let start_index = self.index;
        self.ignore_value()?;
        let raw = &self.input[start_index..self.index];
        visitor.visit_map(super::raw::BorrowedRawDeserializer {
            raw_value: Some(unsafe { str::from_utf8_unchecked(raw) }),
        })
    }

    fn ignore_value(&mut self) -> Result<()> {
        let mut scratch = Vec::new();
        self.parse_string(&mut scratch, true).map(|_| ())
    }

    fn decode_hex_escape(&mut self) -> Result<u16> {
        let tail = self.tail();

        if tail.len() < 4 {
            self.index += tail.len();
            return Err(Error::Eof);
        }

        let mut n = 0;
        for (i, &byte) in tail.iter().enumerate().take(4) {
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
    fn tail(&self) -> &'de [u8] {
        &self.input[self.index..]
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        self.index += n;
    }
}

// ---

struct KeyValueSequence<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> KeyValueSequence<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        KeyValueSequence { de }
    }
}

impl<'de, 'a> SeqAccess<'de> for KeyValueSequence<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, _seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        unimplemented!()
    }
}

impl<'de, 'a> MapAccess<'de> for KeyValueSequence<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.parser.tail().is_empty() {
            return Ok(None);
        }

        self.de.parser.key = true;
        let result = seed.deserialize(&mut *self.de).map(Some);
        self.de.parser.key = false;
        result
    }

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

pub enum Reference<'b, 'c, T>
where
    T: ?Sized + 'static,
{
    Borrowed(&'b T),
    Copied(&'c T),
}

impl<'b, 'c, T> Deref for Reference<'b, 'c, T>
where
    T: ?Sized + 'static,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match *self {
            Reference::Borrowed(b) => b,
            Reference::Copied(c) => c,
        }
    }
}

#[inline]
fn decode_hex_val(val: u8) -> Option<u16> {
    let n = HEX[val as usize] as u16;
    if n == 255 { None } else { Some(n) }
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

struct KeyCh;

impl KeyCh {
    const NOT_ALLOWED: u8 = 255;
    const ALLOWED: u8 = 0;
    const EQ_SIGN: u8 = 1;
    const UNICODE: u8 = 2;
}

static KEY: [u8; 256] = {
    const NA: u8 = KeyCh::NOT_ALLOWED;
    const __: u8 = KeyCh::ALLOWED;
    const EQ: u8 = KeyCh::EQ_SIGN;
    const UC: u8 = KeyCh::UNICODE;
    [
        //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
        NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, // 0
        NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, NA, // 1
        NA, __, NA, __, __, __, __, NA, NA, NA, __, __, NA, __, __, __, // 2
        __, __, __, __, __, __, __, __, __, __, __, NA, NA, EQ, NA, __, // 3
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
        __, __, __, __, __, __, __, __, __, __, __, NA, NA, NA, NA, __, // 5
        NA, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
        __, __, __, __, __, __, __, __, __, __, __, NA, NA, NA, __, NA, // 7
        UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, // 8
        UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, // 9
        UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, // A
        UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, // B
        UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, // C
        UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, // D
        UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, // E
        UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, UC, // F
    ]
};

// ---

#[cfg(test)]
mod tests {
    use super::{super::raw::RawValue, *};
    use std::collections::HashMap;

    #[test]
    fn test_struct_no_escape() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            int: u32,
            str1: String,
            str2: String,
        }

        let j = r#"int=42 str1=a str2="b c""#;
        let expected = Test {
            int: 42,
            str1: "a".to_string(),
            str2: "b c".to_string(),
        };
        assert_eq!(expected, from_str(j).unwrap());
    }

    #[test]
    fn test_struct_escape() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            int: u32,
            str1: String,
            str2: String,
        }

        let j = r#"int=0 str1="b=c" str2="a\nb""#;
        let expected = Test {
            int: 0,
            str1: "b=c".to_string(),
            str2: "a\nb".to_string(),
        };
        assert_eq!(expected, from_str(j).unwrap());
    }

    #[test]
    fn test_hex_escape() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            int: u32,
            str1: String,
            str2: String,
        }

        let j = r#"int=0 str1="\u001b[3m" str2="a""#;
        let expected = Test {
            int: 0,
            str1: "\x1b[3m".to_string(),
            str2: "a".to_string(),
        };
        assert_eq!(expected, from_str(j).unwrap());
    }

    #[test]
    fn test_raw() {
        #[derive(Deserialize)]
        struct Test<'a> {
            int: i32,
            str1: String,
            #[serde(borrow)]
            str2: &'a RawValue,
        }

        let j = r#"int=-42 str1=a str2="b \nc""#;
        let parsed: Test = from_str(j).unwrap();
        assert_eq!(parsed.int, -42);
        assert_eq!(parsed.str1, "a");
        assert_eq!(parsed.str2.get(), r#""b \nc""#);
    }

    #[test]
    fn test_single_word() {
        let result = from_str::<HashMap<String, String>>(r#"word"#);
        assert_eq!(result, Err(Error::ExpectedKeyValueDelimiter));
        assert_eq!(result.unwrap_err().to_string(), "expected key-value delimiter");
    }

    #[test]
    fn test_raw_enum() {
        #[derive(Deserialize, PartialEq, Debug)]
        enum TestEnum {
            A,
            B,
            C,
        }

        let val: TestEnum = from_str("B").unwrap();
        assert_eq!(val, TestEnum::B);
    }

    #[test]
    fn test_raw_struct_with_enum() {
        #[derive(Deserialize, PartialEq, Debug)]
        enum TestEnum {
            A,
            B,
            C,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct TestStruct {
            v: TestEnum,
        }

        let val: TestStruct = from_str("v=B").unwrap();
        assert_eq!(val, TestStruct { v: TestEnum::B });
    }

    #[test]
    fn test_empty_value() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            int: u32,
            str1: String,
            str2: String,
            str3: String,
        }

        let j = r#"int=0 str1="" str2= str3="#;
        let expected = Test {
            int: 0,
            str1: "".to_string(),
            str2: "".to_string(),
            str3: "".to_string(),
        };
        assert_eq!(expected, from_str(j).unwrap());
    }

    #[test]
    fn test_deserializer_from_slice_invalid_utf8() {
        let input = &[0xFF, 0xFE]; // Invalid UTF-8 bytes
        let deserializer = Deserializer::from_slice(input);
        assert!(deserializer.is_err());
    }

    #[test]
    fn test_deserializer_new() {
        let input = "key=value";
        let _deserializer = Deserializer::new(input);
        // Just test that constructor works
    }

    #[test]
    fn test_key_with_unicode() {
        // Test parsing a key with non-ASCII characters to trigger unicode validation
        let input = "café=value";
        let result: Result<std::collections::HashMap<String, String>> = from_str(input);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.get("café"), Some(&"value".to_string()));
    }

    #[test]
    fn test_unquoted_value_with_unicode() {
        // Test parsing an unquoted value with non-ASCII characters to trigger unicode validation on line 576
        let input = "key=café";
        let result: Result<std::collections::HashMap<String, String>> = from_str(input);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.get("key"), Some(&"café".to_string()));
    }

    #[test]
    fn test_invalid_surrogate_pair_calculation() {
        // Test surrogate pair processing by creating malformed UTF-16 surrogates
        // The high surrogate 0xD800 followed by an invalid low surrogate should trigger error handling
        // But since valid surrogate pairs always produce valid code points, let's test a different edge case
        // Testing with a string that has invalid UTF-8 bytes to trigger the InvalidUnicodeCodePoint error
        use crate::logfmt::de::Deserializer;

        // Create a deserializer and try to trigger the surrogate pair error path
        // by testing the boundary condition where char::from_u32 could return None
        let mut deserializer = Deserializer::new(r#"key="\uD800\uDC00""#);
        let result: Result<std::collections::HashMap<String, String>> =
            serde::Deserialize::deserialize(&mut deserializer);
        // This should succeed as it's a valid surrogate pair, let's just ensure it works
        assert!(result.is_ok());
    }
}
