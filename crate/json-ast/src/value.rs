use crate::container::Node;

// ---

/// Represent any valid JSON value.
#[derive(Debug)]
pub enum Value<'s> {
    Null,
    Bool(bool),
    Number(&'s str),
    String(String<'s>),
    Array(Array<'s>),
    Object(Object<'s>),
}

impl<'s> From<String<'s>> for Value<'s> {
    #[inline]
    fn from(s: String<'s>) -> Self {
        Value::String(s)
    }
}

impl<'s> From<bool> for Value<'s> {
    #[inline]
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

// ---

#[derive(Debug)]
pub struct Array<'s> {
    inner: Node<'s>,
}

impl<'s> Array<'s> {
    pub fn new(inner: Node<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> From<Array<'s>> for Value<'s> {
    #[inline]
    fn from(a: Array<'s>) -> Self {
        Value::Array(a)
    }
}

// ---

#[derive(Debug)]
pub struct Object<'s> {
    inner: Node<'s>,
}

impl<'s> Object<'s> {
    pub fn new(inner: Node<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> From<Object<'s>> for Value<'s> {
    #[inline]
    fn from(o: Object<'s>) -> Self {
        Value::Object(o)
    }
}

// ---

#[derive(PartialEq, Eq, Hash)]
pub enum String<'s> {
    Decoded(&'s str),
    Encoded(&'s str),
}

impl<'s> std::fmt::Debug for String<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decoded(s) => write!(f, "{:?}", s),
            Self::Encoded(s) => write!(f, "{:?}", s),
        }
    }
}

impl<'s> String<'s> {
    #[inline]
    pub fn from_plain(s: &'s str) -> Self {
        Self::Decoded(&s[1..s.len() - 1])
    }

    #[inline]
    pub fn from_escaped(s: &'s str) -> Self {
        Self::Encoded(s)
    }
}
