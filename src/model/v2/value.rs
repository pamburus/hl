// workspace imports
use encstr::EncodedString;

// local imports
use super::ast::{self, Composite};

// ---

#[derive(Debug, Clone, Copy)]
pub enum Value<'s> {
    Null,
    Boolean(bool),
    Number(&'s str),
    String(EncodedString<'s>),
    Array(Array<'s>),
    Object(Object<'s>),
}

impl<'s> Value<'s> {
    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Null => true,
            Self::Boolean(_) => false,
            Self::Number(s) => s.is_empty(),
            Self::String(s) => s.is_empty(),
            Self::Array(a) => a.len() == 0,
            Self::Object(o) => o.len() == 0,
        }
    }

    #[inline]
    pub fn as_text(&self) -> Option<EncodedString<'s>> {
        match self {
            Self::Null => Some(EncodedString::raw("null")),
            Self::Boolean(true) => Some(EncodedString::raw("true")),
            Self::Boolean(false) => Some(EncodedString::raw("false")),
            Self::Number(s) => Some(EncodedString::raw(s)),
            Self::String(s) => Some(*s),
            Self::Array(_) => None,
            Self::Object(_) => None,
        }
    }
}

impl<'s> From<ast::Node<'s>> for Value<'s> {
    #[inline]
    fn from(node: ast::Node<'s>) -> Self {
        match *node.value() {
            ast::Value::Scalar(scalar) => scalar.into(),
            ast::Value::Composite(composite) => match composite {
                ast::Composite::Array => Self::Array(Array::new(node)),
                ast::Composite::Object => Self::Object(Object::new(node)),
                ast::Composite::Field(_) => panic!("expected scalar, array or object node, got {:?}", node),
            },
        }
    }
}

impl<'s> From<ast::Scalar<'s>> for Value<'s> {
    #[inline]
    fn from(value: ast::Scalar<'s>) -> Self {
        match value {
            ast::Scalar::Null => Self::Null,
            ast::Scalar::Bool(b) => Self::Boolean(b),
            ast::Scalar::Number(s) => Self::Number(s),
            ast::Scalar::String(s) => Self::String(s.into()),
        }
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Array<'s> {
    inner: ast::Node<'s>,
}

impl<'s> Array<'s> {
    #[inline]
    fn new(inner: ast::Node<'s>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn iter(&self) -> ArrayIter<'s> {
        ArrayIter::new(self.inner.children().iter())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.children().len()
    }
}

impl<'s> From<Array<'s>> for Value<'s> {
    #[inline]
    fn from(a: Array<'s>) -> Self {
        Value::Array(a)
    }
}

impl<'s> IntoIterator for Array<'s> {
    type Item = Value<'s>;
    type IntoIter = ArrayIter<'s>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ArrayIter::new(self.inner.children().iter())
    }
}

// ---

pub struct ArrayIter<'s> {
    inner: ast::SiblingsIter<'s>,
}

impl<'s> ArrayIter<'s> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> Iterator for ArrayIter<'s> {
    type Item = Value<'s>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Value::from)
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Object<'s> {
    inner: ast::Node<'s>,
}

impl<'s> Object<'s> {
    #[inline]
    fn new(inner: ast::Node<'s>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn iter(&self) -> ObjectIter<'s> {
        ObjectIter::new(self.inner.children().iter())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.children().len()
    }
}

impl<'s> From<Object<'s>> for Value<'s> {
    #[inline]
    fn from(o: Object<'s>) -> Self {
        Value::Object(o)
    }
}

impl<'s> IntoIterator for Object<'s> {
    type Item = Field<'s>;
    type IntoIter = ObjectIter<'s>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ObjectIter::new(self.inner.children().iter())
    }
}

// ---

pub struct ObjectIter<'s> {
    inner: ast::SiblingsIter<'s>,
}

impl<'s> ObjectIter<'s> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> Iterator for ObjectIter<'s> {
    type Item = Field<'s>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Field::from_node)
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Field<'s> {
    pub key: &'s str,
    pub value: Value<'s>,
}

impl<'s> Field<'s> {
    #[inline]
    pub fn new(key: &'s str, value: Value<'s>) -> Self {
        Self { key, value }
    }

    #[inline]
    pub(super) fn from_node(node: ast::Node<'s>) -> Self {
        let ast::Value::Composite(Composite::Field(key)) = node.value() else {
            panic!("expected field node, got {:?}", node.value());
        };

        Field {
            key: key.source(),
            value: node.children().iter().next().map(|x| x.into()).unwrap_or(Value::Null),
        }
    }
}
