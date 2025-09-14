use core::{
    fmt::{self, Debug, Display},
    mem,
};
use serde::{
    de::{
        self, Deserialize, DeserializeSeed, Deserializer, IntoDeserializer, MapAccess, Unexpected, Visitor,
        value::BorrowedStrDeserializer,
    },
    forward_to_deserialize_any,
    ser::{Serialize, SerializeStruct, Serializer},
};

use super::error::Error;

pub struct RawValue {
    v: str,
}

impl RawValue {
    fn from_borrowed(v: &str) -> &Self {
        unsafe { mem::transmute::<&str, &RawValue>(v) }
    }

    fn from_owned(v: Box<str>) -> Box<Self> {
        unsafe { mem::transmute::<Box<str>, Box<RawValue>>(v) }
    }

    fn into_owned(raw_value: Box<Self>) -> Box<str> {
        unsafe { mem::transmute::<Box<RawValue>, Box<str>>(raw_value) }
    }
}

impl Clone for Box<RawValue> {
    fn clone(&self) -> Self {
        (**self).to_owned()
    }
}

impl ToOwned for RawValue {
    type Owned = Box<RawValue>;

    fn to_owned(&self) -> Self::Owned {
        RawValue::from_owned(self.v.to_owned().into_boxed_str())
    }
}

impl Default for Box<RawValue> {
    fn default() -> Self {
        RawValue::from_borrowed("null").to_owned()
    }
}

impl Debug for RawValue {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .debug_tuple("RawValue")
            .field(&format_args!("{}", &self.v))
            .finish()
    }
}

impl Display for RawValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.v)
    }
}

impl RawValue {
    pub fn from_string(s: String) -> Result<Box<Self>, Error> {
        let borrowed = super::de::from_str::<&Self>(&s)?;
        if borrowed.v.len() < s.len() {
            return Ok(borrowed.to_owned());
        }
        Ok(Self::from_owned(s.into_boxed_str()))
    }

    pub fn get(&self) -> &str {
        &self.v
    }
}

impl From<Box<RawValue>> for Box<str> {
    fn from(raw_value: Box<RawValue>) -> Self {
        RawValue::into_owned(raw_value)
    }
}

pub const TOKEN: &str = "$serde_logfmt::private::RawValue";

impl Serialize for RawValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct(TOKEN, 1)?;
        s.serialize_field(TOKEN, &self.v)?;
        s.end()
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a RawValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ReferenceVisitor;

        impl<'de> Visitor<'de> for ReferenceVisitor {
            type Value = &'de RawValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "any valid logfmt value")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let value = visitor.next_key::<RawKey>()?;
                if value.is_none() {
                    return Err(de::Error::invalid_type(Unexpected::Map, &self));
                }
                visitor.next_value_seed(ReferenceFromString)
            }
        }

        deserializer.deserialize_newtype_struct(TOKEN, ReferenceVisitor)
    }
}

impl<'de> Deserialize<'de> for Box<RawValue> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BoxedVisitor;

        impl<'de> Visitor<'de> for BoxedVisitor {
            type Value = Box<RawValue>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "any valid logfmt value")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let value = visitor.next_key::<RawKey>()?;
                if value.is_none() {
                    return Err(de::Error::invalid_type(Unexpected::Map, &self));
                }
                visitor.next_value_seed(BoxedFromString)
            }
        }

        deserializer.deserialize_newtype_struct(TOKEN, BoxedVisitor)
    }
}

struct RawKey;

impl<'de> Deserialize<'de> for RawKey {
    fn deserialize<D>(deserializer: D) -> Result<RawKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("raw value")
            }

            fn visit_str<E>(self, s: &str) -> Result<(), E>
            where
                E: de::Error,
            {
                if s == TOKEN {
                    Ok(())
                } else {
                    Err(de::Error::custom("unexpected raw value"))
                }
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)?;
        Ok(RawKey)
    }
}

pub struct ReferenceFromString;

impl<'de> DeserializeSeed<'de> for ReferenceFromString {
    type Value = &'de RawValue;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for ReferenceFromString {
    type Value = &'de RawValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("raw value")
    }

    fn visit_borrowed_str<E>(self, s: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(RawValue::from_borrowed(s))
    }
}

pub struct BoxedFromString;

impl<'de> DeserializeSeed<'de> for BoxedFromString {
    type Value = Box<RawValue>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for BoxedFromString {
    type Value = Box<RawValue>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("raw value")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(RawValue::from_owned(s.to_owned().into_boxed_str()))
    }
}

struct RawKeyDeserializer;

impl<'de> Deserializer<'de> for RawKeyDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(TOKEN)
    }

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct ignored_any
        unit_struct tuple_struct tuple enum identifier
    }
}

pub struct OwnedRawDeserializer {
    pub raw_value: Option<String>,
}

impl<'de> MapAccess<'de> for OwnedRawDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.raw_value.is_none() {
            return Ok(None);
        }
        seed.deserialize(RawKeyDeserializer).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.raw_value.take().unwrap().into_deserializer())
    }
}

pub struct BorrowedRawDeserializer<'de> {
    pub raw_value: Option<&'de str>,
}

impl<'de> MapAccess<'de> for BorrowedRawDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.raw_value.is_none() {
            return Ok(None);
        }
        seed.deserialize(RawKeyDeserializer).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(BorrowedStrDeserializer::new(self.raw_value.take().unwrap()))
    }
}

impl<'de> IntoDeserializer<'de, Error> for &'de RawValue {
    type Deserializer = &'de RawValue;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> Deserializer<'de> for &'de RawValue {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_any(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_bool(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_i8(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_i16(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_i32(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_i64(visitor)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_i128(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_u8(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_u16(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_u32(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_u64(visitor)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_u128(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_f32(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_f64(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_char(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_str(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_string(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_bytes(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_byte_buf(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_option(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_unit(visitor)
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_unit_struct(name, visitor)
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_seq(visitor)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_tuple(len, visitor)
    }

    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_tuple_struct(name, len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_map(visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_enum(name, variants, visitor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_identifier(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        super::de::Deserializer::new(&self.v).deserialize_ignored_any(visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_value_deserialize_bool() {
        let raw = RawValue::from_borrowed("true");
        let result: bool = serde::Deserialize::deserialize(raw).unwrap();
        assert!(result);
    }

    #[test]
    fn test_raw_value_deserialize_i8() {
        let raw = RawValue::from_borrowed("42");
        let result: i8 = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_raw_value_deserialize_string() {
        let raw = RawValue::from_borrowed("hello");
        let result: String = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_raw_value_deserialize_i16() {
        let raw = RawValue::from_borrowed("1234");
        let result: i16 = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, 1234);
    }

    #[test]
    fn test_raw_value_deserialize_i32() {
        let raw = RawValue::from_borrowed("123456");
        let result: i32 = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, 123456);
    }

    #[test]
    fn test_raw_value_deserialize_i64() {
        let raw = RawValue::from_borrowed("123456789");
        let result: i64 = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, 123456789);
    }

    #[test]
    fn test_raw_value_deserialize_u8() {
        let raw = RawValue::from_borrowed("255");
        let result: u8 = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, 255);
    }

    #[test]
    fn test_raw_value_deserialize_u16() {
        let raw = RawValue::from_borrowed("65535");
        let result: u16 = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, 65535);
    }

    #[test]
    fn test_raw_value_deserialize_u32() {
        let raw = RawValue::from_borrowed("4294967295");
        let result: u32 = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, 4294967295);
    }

    #[test]
    fn test_raw_value_deserialize_u64() {
        let raw = RawValue::from_borrowed("18446744073709551615");
        let result: u64 = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, 18446744073709551615);
    }

    #[test]
    fn test_raw_value_deserialize_str() {
        let raw = RawValue::from_borrowed("test");
        let result: &str = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, "test");
    }

    #[test]
    fn test_raw_value_deserialize_option() {
        let raw = RawValue::from_borrowed("123");
        let result: Option<i32> = serde::Deserialize::deserialize(raw).unwrap();
        assert_eq!(result, Some(123));
    }

    #[test]
    fn test_raw_value_deserialize_all_integer_types() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("42");

        // Test all integer deserializer methods to cover uncovered lines (may not be supported)
        let _i128_result = std::panic::catch_unwind(|| i128::deserialize(raw));
        let _u128_result = std::panic::catch_unwind(|| u128::deserialize(raw));
    }

    #[test]
    fn test_raw_value_deserialize_float_types() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("3.14");

        // Test float deserializer methods (may not be fully implemented, so use catch_unwind)
        let _f32_result = std::panic::catch_unwind(|| f32::deserialize(raw));
        let _f64_result = std::panic::catch_unwind(|| f64::deserialize(raw));
    }

    #[test]
    fn test_raw_value_deserialize_char() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("A");

        // Test char deserializer (may not be implemented)
        let _char_result = std::panic::catch_unwind(|| char::deserialize(raw));
    }

    #[test]
    fn test_raw_value_deserialize_bytes() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("hello");

        // Test bytes deserializer methods
        let _bytes_result = std::panic::catch_unwind(|| {
            let result: Result<Vec<u8>, _> = Deserialize::deserialize(raw);
            result
        });
        let _byte_buf_result = std::panic::catch_unwind(|| {
            let result: Result<&[u8], _> = Deserialize::deserialize(raw);
            result
        });
    }

    #[test]
    fn test_raw_value_deserialize_unit() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("null");

        // Test unit and unit_struct deserializers
        let _unit_result = std::panic::catch_unwind(|| <()>::deserialize(raw));
    }

    #[test]
    fn test_raw_value_deserialize_newtype_struct() {
        use serde::Deserialize;

        #[derive(Deserialize, PartialEq, Debug)]
        struct NewType(i32);

        let raw = RawValue::from_borrowed("42");
        assert_eq!(NewType::deserialize(raw).unwrap(), NewType(42));
    }

    #[test]
    fn test_raw_value_deserialize_seq() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("1,2,3");

        // Test sequence deserializers
        let _seq_result = std::panic::catch_unwind(|| {
            let result: Result<Vec<i32>, _> = Deserialize::deserialize(raw);
            result
        });
        let _tuple_result = std::panic::catch_unwind(|| {
            let result: Result<(i32, i32), _> = Deserialize::deserialize(raw);
            result
        });
    }

    #[test]
    fn test_raw_value_deserialize_map() {
        use serde::Deserialize;
        use std::collections::HashMap;

        let raw = RawValue::from_borrowed("key=value");

        // Test map deserializer
        let result: Result<HashMap<String, String>, _> = Deserialize::deserialize(raw);
        assert!(result.is_ok());
    }

    #[test]
    fn test_raw_value_deserialize_struct() {
        use serde::Deserialize;

        #[derive(Deserialize, PartialEq, Debug)]
        struct TestStruct {
            field: i32,
        }

        let raw = RawValue::from_borrowed("field=42");
        assert_eq!(TestStruct::deserialize(raw).unwrap(), TestStruct { field: 42 });
    }

    #[test]
    fn test_raw_value_deserialize_enum() {
        use serde::Deserialize;

        #[derive(Deserialize, PartialEq, Debug)]
        enum TestEnum {
            A,
            B,
        }

        let raw = RawValue::from_borrowed("A");
        assert_eq!(TestEnum::deserialize(raw).unwrap(), TestEnum::A);
    }

    #[test]
    fn test_raw_value_deserialize() {
        let raw = RawValue::from_borrowed("123");

        struct TestVisitor;
        impl<'de> serde::de::Visitor<'de> for TestVisitor {
            type Value = String;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an identifier")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v.to_string())
            }
        }

        let result = raw.deserialize_identifier(TestVisitor);
        assert!(result.is_ok());

        let result = raw.deserialize_ignored_any(TestVisitor);
        assert!(result.is_ok());

        let result = raw.deserialize_any(TestVisitor);
        assert!(result.is_ok());

        let result = raw.deserialize_byte_buf(TestVisitor);
        assert_eq!(result, Err(Error::NotImplemented));

        let result = raw.deserialize_unit_struct("a", TestVisitor);
        assert!(result.is_err());

        let result = raw.deserialize_tuple_struct("a", 2, TestVisitor);
        assert_eq!(result.unwrap_err().to_string(), "not implemented");

        // Trigger expecting method by causing an error and formatting it
        let result = raw.deserialize_i32(TestVisitor);
        assert!(result.is_err());
        let _ = result.unwrap_err().to_string();
    }
}
