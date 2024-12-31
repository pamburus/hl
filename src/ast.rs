use encstr::EncodedString;
use flat_tree::{tree, FlatTree};
use std::ops::Range;

// ---

pub type Span = Range<usize>;

pub mod error {
    pub use super::Span;
    pub type Error = (&'static str, Span);
    pub type Result<T> = std::result::Result<T, Error>;
}

use error::Result;

#[derive(Default, Debug)]
pub struct Container<'s> {
    pub inner: ContainerInner<'s>,
}

impl<'s> Container<'s> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn nodes(&self) -> tree::Nodes<Value<'s>> {
        self.inner.nodes()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    #[inline]
    pub fn metaroot(&mut self) -> tree::NodeBuilder<Value<'s>> {
        self.inner.metaroot()
    }
}

// ---

pub trait Build<'s>: tree::Build<Value = Value<'s>> {}
pub type Children<'s> = tree::Children<'s, Value<'s>>;

impl<'s, T: tree::Build<Value = Value<'s>>> Build<'s> for T {}

pub trait BuildExt<'s>: Build<'s> {
    fn add_scalar(self, scalar: Scalar<'s>) -> Self;
    fn add_object(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_array(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_field(self, key: String<'s>, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
}

impl<'s, T> BuildExt<'s> for T
where
    T: Build<'s>,
{
    #[inline]
    fn add_scalar(self, scalar: Scalar<'s>) -> Self {
        self.push(Value::Scalar(scalar))
    }

    #[inline]
    fn add_object(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Value::Object, f)
    }

    #[inline]
    fn add_array(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Value::Array, f)
    }

    #[inline]
    fn add_field(self, key: String<'s>, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Value::Key(key), f)
    }
}

// ---

pub type ContainerInner<'s> = FlatTree<Value<'s>>;
pub type SiblingsIter<'s> = tree::SiblingsIter<'s, Value<'s>>;
pub type Node<'s> = tree::Node<'s, Value<'s>>;
pub type String<'s> = EncodedString<'s>;

// ---

#[derive(Debug, Clone, Copy)]
pub enum Value<'s> {
    Scalar(Scalar<'s>),
    Array,
    Object,
    Key(String<'s>),
}

#[derive(Debug, Clone, Copy)]
pub enum Scalar<'s> {
    Null,
    Bool(bool),
    Number(&'s str),
    String(String<'s>),
}
