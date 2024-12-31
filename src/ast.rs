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
    pub fn nodes(&self) -> tree::Nodes<Node<'s>> {
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
    pub fn metaroot(&mut self) -> tree::NodeBuilder<Node<'s>> {
        self.inner.metaroot()
    }
}

// ---

pub trait Build<'s>: tree::Build<Value = Node<'s>> {}

impl<'s, T: tree::Build<Value = Node<'s>>> Build<'s> for T {}

pub trait BuildExt<'s>: Build<'s> {
    fn add_scalar(self, source: &'s str, kind: Scalar) -> Self;
    fn add_object(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_array(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_field(
        self,
        key: &'s str,
        key_kind: String,
        f: impl FnOnce(Self::Child) -> Result<Self::Child>,
    ) -> Result<Self>;
}

impl<'s, T> BuildExt<'s> for T
where
    T: Build<'s>,
{
    #[inline]
    fn add_scalar(self, source: &'s str, kind: Scalar) -> Self {
        self.push(Node::new(NodeKind::Scalar(kind), source))
    }

    #[inline]
    fn add_object(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Node::new(NodeKind::Object, ""), f)
    }

    #[inline]
    fn add_array(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Node::new(NodeKind::Array, ""), f)
    }

    #[inline]
    fn add_field(
        self,
        key: &'s str,
        key_kind: String,
        f: impl FnOnce(Self::Child) -> Result<Self::Child>,
    ) -> Result<Self> {
        self.build_e(Node::new(NodeKind::Field(key_kind), key), f)
    }
}

// ---

pub type ContainerInner<'s> = FlatTree<Node<'s>>;

// ---

#[derive(Debug)]
pub struct Node<'s> {
    kind: NodeKind,
    source: &'s str,
}

impl<'s> Node<'s> {
    #[inline]
    pub fn new(kind: NodeKind, source: &'s str) -> Self {
        Self { kind, source }
    }
}

#[derive(Debug)]
pub enum NodeKind {
    Scalar(Scalar),
    Array,
    Object,
    Field(String),
}

#[derive(Debug)]
pub enum Scalar {
    Null,
    Bool(bool),
    Number,
    String(String),
}

#[derive(Debug)]
pub enum String {
    Plain,
    Escaped,
}
