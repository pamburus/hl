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

pub use error::Result;

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
    pub fn roots(&self) -> tree::Roots<Value<'s>> {
        self.inner.roots()
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

pub trait Build<'s>: tree::BuildE<Value = Value<'s>> {}
pub type Children<'s> = tree::Children<'s, Value<'s>>;

impl<'s, T: tree::BuildE<Value = Value<'s>>> Build<'s> for T {}

pub trait BuildExt<'s>
where
    Self: Sized,
{
    type Child: BuildExt<'s>;

    fn add_scalar(self, scalar: Scalar<'s>) -> Self;
    fn add_composite(
        self,
        composite: Composite<'s>,
        f: impl FnOnce(Self::Child) -> Result<Self::Child>,
    ) -> Result<Self>;
}

impl<'s, T> BuildExt<'s> for T
where
    T: tree::BuildE<Value = Value<'s>>,
{
    type Child = T::Child;

    #[inline]
    fn add_scalar(self, scalar: Scalar<'s>) -> Self {
        self.push(Value::Scalar(scalar))
    }

    #[inline]
    fn add_composite(
        self,
        composite: Composite<'s>,
        f: impl FnOnce(Self::Child) -> Result<Self::Child>,
    ) -> Result<Self> {
        self.build_e(composite.into(), f)
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
    Composite(Composite<'s>),
}

impl<'s> Value<'s> {
    #[inline]
    pub const fn null() -> Self {
        Self::Scalar(Scalar::Null)
    }

    #[inline]
    pub const fn bool(value: bool) -> Self {
        Self::Scalar(Scalar::Bool(value))
    }

    #[inline]
    pub const fn number(value: &'s str) -> Self {
        Self::Scalar(Scalar::Number(value))
    }

    #[inline]
    pub const fn string(value: String<'s>) -> Self {
        Self::Scalar(Scalar::String(value))
    }

    #[inline]
    pub const fn array() -> Self {
        Self::Composite(Composite::Array)
    }

    #[inline]
    pub const fn object() -> Self {
        Self::Composite(Composite::Object)
    }

    #[inline]
    pub const fn field(value: String<'s>) -> Self {
        Self::Composite(Composite::Field(value))
    }
}

impl<'s> From<Scalar<'s>> for Value<'s> {
    #[inline]
    fn from(scalar: Scalar<'s>) -> Self {
        Self::Scalar(scalar)
    }
}

impl<'s> From<Composite<'s>> for Value<'s> {
    #[inline]
    fn from(composite: Composite<'s>) -> Self {
        Self::Composite(composite)
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub enum Scalar<'s> {
    Null,
    Bool(bool),
    Number(&'s str),
    String(String<'s>),
}

#[derive(Debug, Clone, Copy)]
pub enum Composite<'s> {
    Array,
    Object,
    Field(String<'s>),
}
