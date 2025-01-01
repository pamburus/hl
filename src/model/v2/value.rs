// workspace imports
use encstr::EncodedString;

// local imports
use super::ast::{self, Composite};

// ---

#[derive(Debug, Clone, Copy)]
pub enum Value<'s> {
    Null,
    Bool(bool),
    Number(&'s str),
    String(EncodedString<'s>),
    Array(Array<'s>),
    Object(Object<'s>),
}

impl<'s> From<ast::Node<'s>> for Value<'s> {
    fn from(node: ast::Node<'s>) -> Self {
        match *node.value() {
            ast::Value::Scalar(scalar) => match scalar {
                ast::Scalar::Null => Self::Null,
                ast::Scalar::Bool(b) => Self::Bool(b),
                ast::Scalar::Number(s) => Self::Number(s),
                ast::Scalar::String(s) => Self::String(s.into()),
            },
            ast::Value::Composite(composite) => match composite {
                ast::Composite::Array => Self::Array(Array::new(node)),
                ast::Composite::Object => Self::Object(Object::new(node)),
                ast::Composite::Field(_) => panic!("expected scalar, array or object node, got {:?}", node),
            },
        }
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Array<'s> {
    inner: ast::Node<'s>,
}

impl<'s> Array<'s> {
    fn new(inner: ast::Node<'s>) -> Self {
        Self { inner }
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
    fn new(inner: ast::Node<'s>) -> Self {
        Self { inner }
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
            value: node.children().iter().next().unwrap().into(),
        }
    }
}
