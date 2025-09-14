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
