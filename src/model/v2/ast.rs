// std imports
use std::{fmt::Debug, ops::Range};

// third-party imports
use derive_where::derive_where;

// workspace imports
use encstr::EncodedString;
use flat_tree::{
    tree::{self, NoAttachment},
    FlatTree,
};

// ---

const DEFAULT_STORAGE_CAPACITY: usize = 128;

pub type Span = Range<usize>;

pub mod error {
    pub use super::Span;
    pub type Error = (&'static str, Span);
    pub type Result<T> = std::result::Result<T, Error>;
}

pub use error::Result;

#[derive(Default, Debug)]
pub struct Container<'s, const N: usize = DEFAULT_STORAGE_CAPACITY> {
    pub inner: ContainerInner<'s, N>,
}

impl<'s> Container<'s> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'s, const N: usize> Container<'s, N> {
    #[inline]
    pub fn roots(&self) -> tree::Roots<Value<'s>, Storage<'s, N>> {
        self.inner.roots()
    }

    #[inline]
    pub fn nodes(&self) -> tree::Nodes<Value<'s>, Storage<'s, N>> {
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
    pub fn metaroot(&mut self) -> Builder<tree::NodeBuilder<Value<'s>, Storage<'s, N>>> {
        Builder::new(self.inner.metaroot())
    }
}

// ---

trait InnerBuild<'s>: tree::Build<Value = Value<'s>> {}
impl<'s, T: tree::Build<Value = Value<'s>>> InnerBuild<'s> for T {}

pub trait BuildAttachment: tree::BuildAttachment {}
impl<A: tree::BuildAttachment> BuildAttachment for A {}

pub type Children<'s, const N: usize = DEFAULT_STORAGE_CAPACITY> = tree::Children<'s, Value<'s>, Storage<'s, N>>;
pub use tree::{AttachmentChild, AttachmentParent, AttachmentValue};

pub trait Build<'s>
where
    Self: Sized,
{
    type Child: Build<'s, Attachment = Self::Attachment>;
    type Attachment: BuildAttachment;
    type WithAttachment<V>: Build<
        's,
        Attachment = AttachmentChild<Self::Attachment, V>,
        WithoutAttachment = Self,
        Child = <Self::Child as Build<'s>>::WithAttachment<V>,
    >;
    type WithoutAttachment: Build<'s, Attachment = AttachmentParent<Self::Attachment>>;

    fn add_scalar(self, scalar: Scalar<'s>) -> Self;
    fn add_composite<F>(self, composite: Composite<'s>, f: F) -> Result<Self>
    where
        F: FnOnce(Self::Child) -> Result<Self::Child>;

    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V>;
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>);
}

// ---

pub struct Builder<B> {
    inner: B,
}

impl<B> Builder<B> {
    fn new(inner: B) -> Self {
        Self { inner }
    }
}

impl<'s, T> Build<'s> for Builder<T>
where
    T: InnerBuild<'s>,
{
    type Child = Builder<T::Child>;
    type Attachment = T::Attachment;
    type WithAttachment<V> = Builder<T::WithAttachment<V>>;
    type WithoutAttachment = Builder<T::WithoutAttachment>;

    #[inline]
    fn add_scalar(self, scalar: Scalar<'s>) -> Self {
        Builder::new(self.inner.push(Value::Scalar(scalar)))
    }

    #[inline]
    fn add_composite<F>(self, composite: Composite<'s>, f: F) -> Result<Self>
    where
        F: FnOnce(Self::Child) -> Result<Self::Child>,
    {
        let result = self
            .inner
            .build(composite.into(), |b| f(Builder::new(b)).map(|b| b.inner))?;
        Ok(Builder::new(result))
    }

    #[inline]
    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V> {
        Builder::new(self.inner.attach(attachment))
    }

    #[inline]
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>) {
        let (parent, value) = self.inner.detach();
        (Builder::new(parent), value)
    }
}

// ---

pub struct Discarder<A = NoAttachment>(pub A);

impl Default for Discarder<NoAttachment> {
    #[inline]
    fn default() -> Self {
        Self(NoAttachment)
    }
}

impl<'s, A> Build<'s> for Discarder<A>
where
    A: BuildAttachment,
{
    type Child = Self;
    type Attachment = A;
    type WithAttachment<V> = Discarder<AttachmentChild<A, V>>;
    type WithoutAttachment = Discarder<AttachmentParent<A>>;

    #[inline]
    fn add_scalar(self, _: Scalar<'s>) -> Self {
        self
    }

    #[inline]
    fn add_composite<F>(self, _: Composite<'s>, _: F) -> Result<Self>
    where
        F: FnOnce(Self::Child) -> Result<Self::Child>,
    {
        Ok(self)
    }

    #[inline]
    fn attach<V>(self, value: V) -> Self::WithAttachment<V> {
        Discarder(self.0.join(value))
    }

    #[inline]
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>) {
        let (attachment, value) = self.0.split();
        (Discarder(attachment), value)
    }
}

// ---

pub type ContainerInner<'s, const N: usize = DEFAULT_STORAGE_CAPACITY> = FlatTree<Value<'s>, Storage<'s, N>>;
pub type SiblingsIter<'s, const N: usize = DEFAULT_STORAGE_CAPACITY> =
    tree::SiblingsIter<'s, Value<'s>, Storage<'s, N>>;
pub type Node<'s, const N: usize = DEFAULT_STORAGE_CAPACITY> = tree::Node<'s, Value<'s>, Storage<'s, N>>;
pub type String<'s> = EncodedString<'s>;
pub type Storage<'s, const N: usize = DEFAULT_STORAGE_CAPACITY> = InnerStorage<Value<'s>, N>;

#[derive(Debug)]
#[derive_where(Default)]
pub struct InnerStorage<V, const N: usize = DEFAULT_STORAGE_CAPACITY> {
    buf: heapopt::Vec<tree::Item<V>, N>,
}

impl<V: Debug, const N: usize> flat_tree::Storage for InnerStorage<V, N> {
    type Value = V;

    #[inline]
    fn len(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    fn get(&self, index: usize) -> Option<&tree::Item<Self::Value>> {
        self.buf.get(index)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut tree::Item<Self::Value>> {
        self.buf.get_mut(index)
    }

    #[inline]
    fn push(&mut self, item: tree::Item<Self::Value>) {
        self.buf.push(item)
    }

    #[inline]
    fn clear(&mut self) {
        self.buf.clear();
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let mut container = Container::new();
        let root = container.metaroot();
        root.add_scalar(Scalar::Bool(true))
            .add_composite(Composite::Array, |b| Ok(b.add_scalar(Scalar::Bool(false))))
            .unwrap();
        assert_eq!(container.roots().len(), 2);
    }

    #[test]
    fn test_builder_attach() {
        let mut container = Container::new();
        let root = container.metaroot();
        let attachment = root
            .add_scalar(Scalar::Bool(true))
            .attach("attachment")
            .add_composite(Composite::Array, |b| {
                let (b, attachment) = b.detach();
                assert_eq!(attachment, "attachment");
                Ok(b.add_scalar(Scalar::Bool(false)).attach("another attachment"))
            })
            .unwrap()
            .detach()
            .1;
        assert_eq!(container.roots().len(), 2);
        assert_eq!(attachment, "another attachment");
    }
}
