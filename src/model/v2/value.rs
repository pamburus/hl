// workspace imports
use encstr::EncodedString;

// local imports
use super::ast::{self, Composite};

// ---

#[derive(Debug, Clone, Copy)]
pub enum Value<'r, 's> {
    Null,
    Boolean(bool),
    Number(&'s str),
    String(EncodedString<'s>),
    Array(Array<'r, 's>),
    Object(Object<'r, 's>),
}

impl<'r, 's> Value<'r, 's> {
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

impl<'r, 's> From<ast::Node<'r, 's>> for Value<'r, 's> {
    #[inline]
    fn from(node: ast::Node<'r, 's>) -> Self {
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

impl<'s> From<ast::Scalar<'s>> for Value<'_, 's> {
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

impl<'r, 's> TryInto<ast::Scalar<'s>> for Value<'r, 's> {
    type Error = Self;

    #[inline]
    fn try_into(self: Self) -> Result<ast::Scalar<'s>, Self> {
        match self {
            Self::Null => Ok(ast::Scalar::Null),
            Self::Boolean(b) => Ok(ast::Scalar::Bool(b)),
            Self::Number(s) => Ok(ast::Scalar::Number(s)),
            Self::String(s) => Ok(ast::Scalar::String(s.into())),
            _ => Err(self),
        }
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Array<'r, 's> {
    inner: ast::Node<'r, 's>,
}

impl<'r, 's> Array<'r, 's> {
    #[inline]
    fn new(inner: ast::Node<'r, 's>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn iter(&self) -> ArrayIter<'r, 's> {
        ArrayIter::new(self.inner.children().iter())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.children().len()
    }
}

impl<'r, 's> From<Array<'r, 's>> for Value<'r, 's> {
    #[inline]
    fn from(a: Array<'r, 's>) -> Self {
        Value::Array(a)
    }
}

impl<'r, 's> IntoIterator for Array<'r, 's> {
    type Item = Value<'r, 's>;
    type IntoIter = ArrayIter<'r, 's>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ArrayIter::new(self.inner.children().iter())
    }
}

// ---

pub struct ArrayIter<'r, 's> {
    inner: ast::SiblingsIter<'r, 's>,
}

impl<'r, 's> ArrayIter<'r, 's> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'r, 's>) -> Self {
        Self { inner }
    }
}

impl<'r, 's> Iterator for ArrayIter<'r, 's> {
    type Item = Value<'r, 's>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Value::from)
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Object<'r, 's> {
    inner: ast::Node<'r, 's>,
}

impl<'r, 's> Object<'r, 's> {
    #[inline]
    fn new(inner: ast::Node<'r, 's>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn iter(&self) -> ObjectIter<'r, 's> {
        ObjectIter::new(self.inner.children().iter())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.children().len()
    }
}

impl<'r, 's> From<Object<'r, 's>> for Value<'r, 's> {
    #[inline]
    fn from(o: Object<'r, 's>) -> Self {
        Value::Object(o)
    }
}

impl<'r, 's> IntoIterator for Object<'r, 's> {
    type Item = Field<'r, 's>;
    type IntoIter = ObjectIter<'r, 's>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ObjectIter::new(self.inner.children().iter())
    }
}

// ---

pub struct ObjectIter<'r, 's> {
    inner: ast::SiblingsIter<'r, 's>,
}

impl<'r, 's> ObjectIter<'r, 's> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'r, 's>) -> Self {
        Self { inner }
    }
}

impl<'r, 's> Iterator for ObjectIter<'r, 's> {
    type Item = Field<'r, 's>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Field::from_node)
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Field<'r, 's> {
    pub key: &'s str,
    pub value: Value<'r, 's>,
}

impl<'r, 's> Field<'r, 's> {
    #[inline]
    pub fn new(key: &'s str, value: Value<'r, 's>) -> Self {
        Self { key, value }
    }

    #[inline]
    pub(super) fn from_node(node: ast::Node<'r, 's>) -> Self {
        let ast::Value::Composite(Composite::Field(key)) = node.value() else {
            panic!("expected field node, got {:?}", node.value());
        };

        Field {
            key: key.source(),
            value: node.children().iter().next().map(|x| x.into()).unwrap_or(Value::Null),
        }
    }
}
