// std imports
use std::{fmt::Debug, ops::Range};

// third-party imports
use derive_where::derive_where;

// workspace imports
use encstr::EncodedString;
use flat_tree::{
    FlatTree,
    storage::DefaultStorage,
    tree::{self, NoAttachment},
};

// ---

const PREALLOCATED_CAPACITY: usize = 128;

pub type Span = Range<usize>;

pub mod error {
    pub use super::Span;
    pub type Error = (&'static str, Span);
    pub type Result<T> = std::result::Result<T, Error>;
}

pub use error::Result;

#[derive(Default, Debug, Clone)]
pub struct Container<'s> {
    pub inner: ContainerInner<'s>,
}

impl<'s> Container<'s> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'s> Container<'s> {
    #[inline]
    pub fn roots(&self) -> tree::Roots<Value<'s>, Storage<'s>> {
        self.inner.roots()
    }

    #[inline]
    pub fn nodes(&self) -> tree::Nodes<Value<'s>, Storage<'s>> {
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
    pub fn metaroot(&mut self) -> Builder<tree::NodeBuilder<Value<'s>, Storage<'s>>> {
        Builder::new(self.inner.metaroot())
    }
}

// ---

trait InnerBuild<'s>: tree::Build<Value = Value<'s>> {}
impl<'s, T: tree::Build<Value = Value<'s>>> InnerBuild<'s> for T {}

pub trait BuildAttachment: tree::BuildAttachment {}
impl<A: tree::BuildAttachment> BuildAttachment for A {}

pub type Children<'c, 's> = tree::Children<'c, Value<'s>, Storage<'s>>;
pub use flat_tree::{Index, OptIndex};
pub use tree::{AttachmentChild, AttachmentParent, AttachmentValue};

pub trait Build<'s>
where
    Self: Sized,
{
    type Child: Build<'s, Checkpoint = Self::Checkpoint, Attachment = Self::Attachment>;
    type Attachment: BuildAttachment;
    type WithAttachment<V>: Build<
            's,
            Attachment = AttachmentChild<Self::Attachment, V>,
            Checkpoint = Self::Checkpoint,
            WithoutAttachment = Self,
            Child = <Self::Child as Build<'s>>::WithAttachment<V>,
        >;
    type WithoutAttachment: Build<'s, Checkpoint = Self::Checkpoint, Attachment = AttachmentParent<Self::Attachment>>;
    type Checkpoint;

    fn add_scalar(self, scalar: Scalar<'s>) -> Self;
    fn add_composite<F>(self, composite: Composite<'s>, f: F) -> (Self, Result<()>)
    where
        F: FnOnce(Self::Child) -> (Self::Child, Result<()>);

    fn checkpoint(&self) -> Self::Checkpoint;
    fn first_node_index(&self, checkpoint: &Self::Checkpoint) -> OptIndex;

    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V>;
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>);
}

pub trait BuildCheckpoint<'s> {
    type Builder: Build<'s>;

    fn first_node_index(&self, builder: &Self::Builder) -> OptIndex;
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
    type Child = Self;
    type Attachment = T::Attachment;
    type WithAttachment<V> = Builder<T::WithAttachment<V>>;
    type WithoutAttachment = Builder<T::WithoutAttachment>;
    type Checkpoint = T::Checkpoint;

    #[inline]
    fn add_scalar(self, scalar: Scalar<'s>) -> Self {
        Builder::new(self.inner.push(Value::Scalar(scalar)))
    }

    #[inline]
    fn add_composite<F>(self, composite: Composite<'s>, f: F) -> (Self, Result<()>)
    where
        F: FnOnce(Self::Child) -> (Self::Child, Result<()>),
    {
        let (b, result) = self.inner.build(composite.into(), |b| {
            let (b, result) = f(Builder::new(b));
            (b.inner, result)
        });
        (Builder::new(b), result)
    }

    #[inline]
    fn checkpoint(&self) -> Self::Checkpoint {
        self.inner.checkpoint()
    }

    #[inline]
    fn first_node_index(&self, checkpoint: &Self::Checkpoint) -> OptIndex {
        self.inner.first_node_index(checkpoint)
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
    type Checkpoint = ();

    #[inline]
    fn add_scalar(self, _: Scalar<'s>) -> Self {
        self
    }

    #[inline]
    fn add_composite<F>(self, _: Composite<'s>, _: F) -> (Self, Result<()>)
    where
        F: FnOnce(Self::Child) -> (Self::Child, Result<()>),
    {
        (self, Ok(()))
    }

    #[inline]
    fn checkpoint(&self) -> Self::Checkpoint {
        ()
    }

    #[inline]
    fn first_node_index(&self, _: &Self::Checkpoint) -> OptIndex {
        None.into()
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

pub type ContainerInner<'s> = FlatTree<Value<'s>, Storage<'s>>;
pub type SiblingsIter<'c, 's> = tree::SiblingsIter<'c, Value<'s>, Storage<'s>>;
pub type Node<'c, 's> = tree::Node<'c, Value<'s>, Storage<'s>>;
pub type String<'s> = EncodedString<'s>;
pub type Storage<'s> = DefaultStorage<Value<'s>>;

#[derive(Debug)]
#[derive_where(Default)]
pub struct HeapOptStorage<V> {
    buf: heapopt::Vec<tree::Item<V>, PREALLOCATED_CAPACITY>,
}

impl<V: Debug> flat_tree::Storage for HeapOptStorage<V> {
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
    fn truncate(&mut self, size: usize) {
        self.buf.truncate(size);
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

impl<'s> Scalar<'s> {
    pub fn as_text(&self) -> EncodedString<'s> {
        match self {
            Self::Null => EncodedString::raw("null"),
            Self::Bool(true) => EncodedString::raw("true"),
            Self::Bool(false) => EncodedString::raw("false"),
            Self::Number(s) => EncodedString::raw(s),
            Self::String(s) => *s,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Composite<'s> {
    Array,
    Object,
    Field(String<'s>),
}

#[cfg(test)]
mod tests;
