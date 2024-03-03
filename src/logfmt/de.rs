use std::{
    ops::{AddAssign, Deref, MulAssign, Neg},
    str,
};

use serde::de::{self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserialize;

use super::error::{Error, Result};

pub struct Deserializer<'de> {
    input: &'de str,
    index: usize,
    scratch: Vec<u8>,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            input,
            index: 0,
            scratch: Vec::new(),
        }
    }
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.tail().is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

impl<'de> Deserializer<'de> {
    fn peek_char(&mut self) -> Result<char> {
        if let Some(ch) = self.tail().chars().next() {
            Ok(ch)
        } else {
            Err(Error::Eof)
        }
    }

    fn next_char(&mut self) -> Result<char> {
        let ch = self.peek_char()?;
        self.advance(ch.len_utf8());
        Ok(ch)
    }

    fn parse_bool(&mut self) -> Result<bool> {
        if self.tail().starts_with("true") {
            self.advance(4);
            Ok(true)
        } else if self.tail().starts_with("false") {
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
        let mut int = match self.next_char()? {
            ch @ '0'..='9' => T::from(ch as u8 - b'0'),
            _ => {
                return Err(Error::ExpectedInteger);
            }
        };
        loop {
            match self.tail().chars().next() {
                Some(ch @ '0'..='9') => {
                    self.advance(1);
                    int *= T::from(10);
                    int += T::from(ch as u8 - b'0');
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
        unimplemented!()
    }

    fn parse_string<'s>(&'s mut self, ignore: bool) -> Result<Reference<'de, 's, str>> {
        if self.peek_char()? != '"' {
            let i = match self.tail().find(|c| c == ' ' || c == '=') {
                Some(len) => len,
                None => self.input.len(),
            };
            let s = &self.tail()[..i];
            self.advance(i);
            return Ok(Reference::Borrowed(s));
        }

        self.next_char()?;
        let mut no_escapes = true;
        if !ignore {
            self.scratch.clear();
        }
        let mut start = self.index;

        loop {
            while self.index < self.input.len() && !ESCAPE[self.input.as_bytes()[self.index] as usize] {
                self.advance(1);
            }
            if self.index == self.input.len() {
                return Err(Error::Eof);
            }
            match self.input.as_bytes()[self.index] {
                b'"' => {
                    if no_escapes {
                        let borrowed = &self.input[start..self.index];
                        self.advance(1);
                        return Ok(Reference::Borrowed(borrowed));
                    }

                    if !ignore {
                        self.scratch
                            .extend_from_slice(&self.input.as_bytes()[start..self.index]);
                    }
                    self.advance(1);

                    return if !ignore {
                        Ok(Reference::Copied(unsafe { str::from_utf8_unchecked(&self.scratch) }))
                    } else {
                        Ok(Reference::Borrowed(&self.input[self.index..self.index]))
                    };
                }
                b'\\' => {
                    no_escapes = false;
                    if !ignore {
                        self.scratch
                            .extend_from_slice(&self.input.as_bytes()[start..self.index]);
                    }
                    self.advance(1);
                    self.parse_escape(ignore)?;
                    start = self.index;
                }
                _ => {
                    self.advance(1);
                    return Err(Error::UnexpectedControlCharacter);
                }
            }
        }
    }

    fn parse_escape(&mut self, ignore: bool) -> Result<()> {
        let ch = self.next_char()?;

        match ch {
            '"' => self.scratch.push(b'"'),
            '\\' => self.scratch.push(b'\\'),
            '/' => self.scratch.push(b'/'),
            'b' => self.scratch.push(b'\x08'),
            'f' => self.scratch.push(b'\x0c'),
            'n' => self.scratch.push(b'\n'),
            'r' => self.scratch.push(b'\r'),
            't' => self.scratch.push(b'\t'),
            'u' => {
                let c = match self.decode_hex_escape()? {
                    0xDC00..=0xDFFF => {
                        return Err(Error::LoneLeadingSurrogateInHexEscape);
                    }

                    n1 @ 0xD800..=0xDBFF => {
                        if self.peek_char()? == '\\' {
                            self.next_char()?;
                        } else {
                            return Err(Error::UnexpectedEndOfHexEscape);
                        }

                        if self.peek_char()? == 'u' {
                            self.next_char()?;
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

                if !ignore {
                    self.scratch.extend_from_slice(c.encode_utf8(&mut [0_u8; 4]).as_bytes());
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
        visitor.visit_map(super::raw::BorrowedRawDeserializer { raw_value: Some(raw) })
    }

    fn ignore_value(&mut self) -> Result<()> {
        self.parse_string(true).map(|_| ())
    }

    fn decode_hex_escape(&mut self) -> Result<u16> {
        if self.input.len() < 4 {
            self.input = &self.input[self.input.len()..];
            return Err(Error::Eof);
        }

        let mut n = 0;
        for i in 0..4 {
            let ch = decode_hex_val(self.input.as_bytes()[i]);
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
    fn tail(&self) -> &'de str {
        &self.input[self.index..]
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        self.index += n;
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_signed()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_signed()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_signed()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_signed()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_unsigned()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_unsigned()?)
    }

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

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parse_string(false)? {
            Reference::Borrowed(b) => visitor.visit_borrowed_str(b),
            Reference::Copied(c) => visitor.visit_str(c),
        }
    }

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
        unimplemented!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.starts_with("null") {
            self.input = &self.input["null".len()..];
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.starts_with("null") {
            self.input = &self.input["null".len()..];
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNull)
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if name == super::raw::TOKEN {
            return self.deserialize_raw_value(visitor);
        }

        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.next_char()? == '[' {
            let value = visitor.visit_seq(SpaceSeparated::new(self))?;
            if self.next_char()? == ']' {
                Ok(value)
            } else {
                Err(Error::ExpectedArrayEnd)
            }
        } else {
            Err(Error::ExpectedArray)
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.peek_char()? == '{' {
            let value = visitor.visit_map(SpaceSeparated::new(self))?;
            if self.next_char()? == '}' {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Ok(visitor.visit_map(SpaceSeparated::new(self))?)
        }
    }

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

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.peek_char()? == '"' {
            visitor.visit_enum(self.parse_string(false)?.into_deserializer())
        } else if self.next_char()? == '{' {
            let value = visitor.visit_enum(Enum::new(self))?;
            if self.next_char()? == '}' {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedEnum)
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct SpaceSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'a, 'de> SpaceSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        SpaceSeparated { de, first: true }
    }
}

impl<'de, 'a> SeqAccess<'de> for SpaceSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.de.peek_char()? == ']' {
            return Ok(None);
        }
        if !self.first && self.de.next_char()? != ' ' {
            return Err(Error::ExpectedArrayDelimiter);
        }
        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'de, 'a> MapAccess<'de> for SpaceSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.tail().len() == 0 || self.de.peek_char()? == '}' {
            return Ok(None);
        }
        if !self.first && self.de.next_char()? != ' ' {
            return Err(Error::ExpectedMapDelimiter);
        }
        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        if self.de.next_char()? != '=' {
            return Err(Error::ExpectedMapKeyValueDelimiter);
        }
        seed.deserialize(&mut *self.de)
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.de)?;
        if self.de.next_char()? == '=' {
            Ok((val, self))
        } else {
            Err(Error::ExpectedMapKeyValueDelimiter)
        }
    }
}

impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Err(Error::ExpectedString)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
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

    fn deref(&self) -> &Self::Target {
        match *self {
            Reference::Borrowed(b) => b,
            Reference::Copied(c) => c,
        }
    }
}

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

// ---

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
fn test_raw() {
    #[derive(Deserialize)]
    struct Test<'a> {
        int: u32,
        str1: String,
        #[serde(borrow)]
        str2: &'a super::raw::RawValue,
    }

    let j = r#"int=42 str1=a str2="b \nc""#;
    let parsed: Test = from_str(j).unwrap();
    assert_eq!(parsed.int, 42);
    assert_eq!(parsed.str1, "a");
    assert_eq!(parsed.str2.get(), r#""b \nc""#);
}
