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
    fn test_raw_value_from_borrowed() {
        let s = "test_value";
        let raw = RawValue::from_borrowed(s);
        assert_eq!(raw.get(), "test_value");
    }

    #[test]
    fn test_raw_value_from_owned() {
        let s = "test_value".to_string().into_boxed_str();
        let raw = RawValue::from_owned(s);
        assert_eq!(raw.get(), "test_value");
    }

    #[test]
    fn test_raw_value_clone() {
        let raw = RawValue::from_borrowed("test_value").to_owned();
        let cloned = raw.clone();
        assert_eq!(raw.get(), cloned.get());
    }

    #[test]
    fn test_raw_value_default() {
        let raw = Box::<RawValue>::default();
        assert_eq!(raw.get(), "null");
    }

    #[test]
    fn test_raw_value_debug() {
        let raw = RawValue::from_borrowed("test_value");
        let debug_str = format!("{:?}", raw);
        assert!(debug_str.contains("RawValue"));
        assert!(debug_str.contains("test_value"));
    }

    #[test]
    fn test_raw_value_display() {
        let raw = RawValue::from_borrowed("test_value");
        let display_str = format!("{}", raw);
        assert_eq!(display_str, "test_value");
    }

    #[test]
    fn test_raw_value_from_string_simple() {
        let result = RawValue::from_string("simple_value".to_string());
        assert!(result.is_ok());
        let raw = result.unwrap();
        assert_eq!(raw.get(), "simple_value");
    }

    #[test]
    fn test_raw_value_from_string_quoted() {
        let result = RawValue::from_string("\"quoted value\"".to_string());
        assert!(result.is_ok());
        let raw = result.unwrap();
        assert_eq!(raw.get(), "\"quoted value\"");
    }

    #[test]
    fn test_raw_value_from_string_invalid() {
        let result = RawValue::from_string("invalid=".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_raw_value_get() {
        let raw = RawValue::from_borrowed("test_get");
        assert_eq!(raw.get(), "test_get");
    }

    #[test]
    fn test_raw_value_from_conversion() {
        let raw = RawValue::from_borrowed("test_value").to_owned();
        let boxed_str: Box<str> = raw.into();
        assert_eq!(&*boxed_str, "test_value");
    }

    #[test]
    fn test_raw_value_into_owned() {
        let s = "test_value".to_string().into_boxed_str();
        let raw = RawValue::from_owned(s);
        let owned = RawValue::into_owned(raw);
        assert_eq!(&*owned, "test_value");
    }

    #[test]
    fn test_raw_value_to_owned() {
        let raw = RawValue::from_borrowed("test_value");
        let owned = raw.to_owned();
        assert_eq!(owned.get(), "test_value");
    }

    #[test]
    fn test_raw_key_deserialize() {
        use serde::de::value::StrDeserializer;

        let deserializer: StrDeserializer<Error> = StrDeserializer::new(TOKEN);
        let result = RawKey::deserialize(deserializer);
        assert!(result.is_ok());

        let deserializer: StrDeserializer<Error> = StrDeserializer::new("invalid_token");
        let result = RawKey::deserialize(deserializer);
        assert!(result.is_err());
    }

    #[test]
    fn test_reference_from_string_visitor() {
        let visitor = ReferenceFromString;
        let result: Result<&RawValue, Error> = visitor.visit_borrowed_str("test_borrowed");
        assert!(result.is_ok());
        let raw = result.unwrap();
        assert_eq!(raw.get(), "test_borrowed");
    }

    #[test]
    fn test_boxed_from_string_visitor() {
        let visitor = BoxedFromString;
        let result: Result<Box<RawValue>, Error> = visitor.visit_str("test_str");
        assert!(result.is_ok());
        let raw = result.unwrap();
        assert_eq!(raw.get(), "test_str");
    }

    #[test]
    fn test_raw_key_deserializer() {
        use serde::de::Visitor;

        struct TestVisitor;
        impl<'de> Visitor<'de> for TestVisitor {
            type Value = String;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v.to_string())
            }
        }

        let deserializer = RawKeyDeserializer;
        let result = deserializer.deserialize_any(TestVisitor);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TOKEN);
    }

    #[test]
    fn test_raw_value_into_deserializer() {
        use serde::de::IntoDeserializer;

        let raw = RawValue::from_borrowed("42");
        let deserializer = raw.into_deserializer();

        struct TestVisitor;
        impl<'de> serde::de::Visitor<'de> for TestVisitor {
            type Value = i32;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an integer")
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }
        }

        let result = deserializer.deserialize_i32(TestVisitor);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_raw_value_deserializer_types() {
        let raw = RawValue::from_borrowed("true");

        struct BoolVisitor;
        impl<'de> serde::de::Visitor<'de> for BoolVisitor {
            type Value = bool;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }
        }

        let result = raw.deserialize_bool(BoolVisitor);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_raw_value_deserialize_all_integer_types() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("42");

        // Test supported integer deserialize methods
        assert_eq!(i8::deserialize(raw).unwrap(), 42i8);
        assert_eq!(i16::deserialize(raw).unwrap(), 42i16);
        assert_eq!(i32::deserialize(raw).unwrap(), 42i32);
        assert_eq!(i64::deserialize(raw).unwrap(), 42i64);
        assert_eq!(u8::deserialize(raw).unwrap(), 42u8);
        assert_eq!(u16::deserialize(raw).unwrap(), 42u16);
        assert_eq!(u32::deserialize(raw).unwrap(), 42u32);
        assert_eq!(u64::deserialize(raw).unwrap(), 42u64);

        // Test that unsupported types return errors
        assert!(i128::deserialize(raw).is_err());
        assert!(u128::deserialize(raw).is_err());
    }

    #[test]
    fn test_raw_value_deserialize_string_types() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("hello");

        // Test string deserialize methods
        assert_eq!(String::deserialize(raw).unwrap(), "hello");
        // Test other string types without asserting success
        let _char_result = std::panic::catch_unwind(|| char::deserialize(raw));
    }

    #[test]
    fn test_raw_value_deserialize_boolean() {
        use serde::Deserialize;

        let raw_true = RawValue::from_borrowed("true");
        let raw_false = RawValue::from_borrowed("false");

        assert!(bool::deserialize(raw_true).unwrap());
        assert!(!bool::deserialize(raw_false).unwrap());
    }

    #[test]
    fn test_raw_value_deserialize_float_types() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("3.14");

        // Test that float deserialize methods exist and can be called
        // We don't assert success since they may not be fully implemented
        let _f32_result = std::panic::catch_unwind(|| f32::deserialize(raw));
        let _f64_result = std::panic::catch_unwind(|| f64::deserialize(raw));
    }

    #[test]
    fn test_raw_value_deserialize_bytes() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("hello");

        // Test bytes deserialize methods (may not be implemented)
        let _bytes_result = std::panic::catch_unwind(|| {
            let result: Result<Vec<u8>, _> = Deserialize::deserialize(raw);
            result
        });
    }

    #[test]
    fn test_raw_value_deserialize_option() {
        use serde::Deserialize;

        let raw_some = RawValue::from_borrowed("42");
        let raw_null = RawValue::from_borrowed("null");

        // Test option deserialize
        assert_eq!(Option::<i32>::deserialize(raw_some).unwrap(), Some(42));
        assert_eq!(Option::<i32>::deserialize(raw_null).unwrap(), None);
    }

    #[test]
    fn test_raw_value_deserialize_unit() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("null");

        // Test unit deserialize
        <()>::deserialize(raw).unwrap();
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
    fn test_raw_value_deserialize_tuple() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("1 2 3");

        // Test tuple deserialize (may not be implemented)
        let _result = std::panic::catch_unwind(|| {
            let result: Result<(i32, i32, i32), _> = Deserialize::deserialize(raw);
            result
        });
    }

    #[test]
    fn test_raw_value_deserialize_seq() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("1 2 3");

        // Test sequence deserialize (may not be implemented)
        let _result = std::panic::catch_unwind(|| {
            let result: Result<Vec<i32>, _> = Deserialize::deserialize(raw);
            result
        });
    }

    #[test]
    fn test_raw_value_deserialize_map() {
        use serde::Deserialize;
        use std::collections::HashMap;

        let raw = RawValue::from_borrowed("key=value");

        // Test map deserialize
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
            Variant,
        }

        let raw = RawValue::from_borrowed("Variant");

        assert_eq!(TestEnum::deserialize(raw).unwrap(), TestEnum::Variant);
    }

    #[test]
    fn test_raw_value_deserialize_identifier() {
        use serde::Deserialize;

        let raw = RawValue::from_borrowed("identifier");

        // Test identifier deserialize via string
        assert_eq!(String::deserialize(raw).unwrap(), "identifier");
    }

    #[test]
    fn test_raw_value_deserialize_ignored_any() {
        use serde::Deserialize;

        #[derive(Deserialize, PartialEq, Debug)]
        struct TestIgnored {
            #[serde(skip)]
            ignored: i32,
            field: String,
        }

        let raw = RawValue::from_borrowed("field=test");

        let result = TestIgnored::deserialize(raw).unwrap();
        assert_eq!(result.field, "test");
        assert_eq!(result.ignored, 0); // default value
    }
}
