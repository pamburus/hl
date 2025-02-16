// std imports
use std::fmt::Debug;

// workspace imports
use flat_tree::{
    tree::{self, NoAttachment},
    FlatTree,
};
use log_format::{ast, origin};

pub use log_format::{
    token::{Composite, Scalar, String},
    Span,
};
pub use origin::Origin;

// ---

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
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: ContainerInner::with_capacity(capacity),
        }
    }
}

impl<'s> Container<'s> {
    #[inline]
    pub fn roots(&self) -> tree::Roots<Value> {
        self.inner.roots()
    }

    #[inline]
    pub fn nodes(&self) -> tree::Nodes<Value> {
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
    pub fn metaroot(&mut self) -> Builder<tree::NodeBuilder<Value<'s>>> {
        Builder::new(self.inner.metaroot())
    }
}

// ---

trait InnerBuild<'s>: tree::Build<Value = Value<'s>> {}
impl<'s, T: tree::Build<Value = Value<'s>>> InnerBuild<'s> for T {}

pub trait BuildAttachment: tree::BuildAttachment {}
impl<A: tree::BuildAttachment> BuildAttachment for A {}

pub type Children<'s, 'c> = tree::Children<'c, Value<'s>>;
pub use flat_tree::{Index, OptIndex};
pub use tree::{AttachmentChild, AttachmentParent, AttachmentValue};

pub trait Build<'s>
where
    Self: Sized,
{
    type Attachment: BuildAttachment;
    type WithAttachment<V>: Build<
        's,
        Attachment = AttachmentChild<Self::Attachment, V>,
        Checkpoint = Self::Checkpoint,
        WithoutAttachment = Self,
    >;
    type WithoutAttachment: Build<'s, Checkpoint = Self::Checkpoint, Attachment = AttachmentParent<Self::Attachment>>;
    type Checkpoint;

    fn add_scalar(self, scalar: Scalar<'s>) -> Self;
    fn add_composite<E, F>(self, composite: Composite<'s>, f: F) -> Result<Self, (E, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (E, Self)>;

    fn checkpoint(&self) -> Self::Checkpoint;
    fn rollback(&mut self, checkpoint: &Self::Checkpoint);
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
    pub fn new(inner: B) -> Self {
        Self { inner }
    }
}

impl<'s, T> Build<'s> for Builder<T>
where
    T: InnerBuild<'s>,
{
    type Attachment = T::Attachment;
    type WithAttachment<V> = Builder<T::WithAttachment<V>>;
    type WithoutAttachment = Builder<T::WithoutAttachment>;
    type Checkpoint = T::Checkpoint;

    #[inline]
    fn add_scalar(self, scalar: Scalar<'s>) -> Self {
        Builder::new(self.inner.push(Value::Scalar(scalar)))
    }

    #[inline]
    fn add_composite<E, F>(self, composite: Composite<'s>, f: F) -> Result<Self, (E, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (E, Self)>,
    {
        match self.inner.build(composite.into(), |b| match f(Builder::new(b)) {
            Ok(b) => Ok(b.inner),
            Err((e, b)) => Err((e, b.inner)),
        }) {
            Ok(b) => Ok(Builder::new(b)),
            Err((e, b)) => Err((e, Builder::new(b))),
        }
    }

    #[inline]
    fn checkpoint(&self) -> Self::Checkpoint {
        self.inner.checkpoint()
    }

    #[inline]
    fn rollback(&mut self, checkpoint: &Self::Checkpoint) {
        self.inner.rollback(checkpoint)
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

impl<'s, T> ast::Build<'s> for Builder<T>
where
    T: InnerBuild<'s>,
{
    type Checkpoint = T::Checkpoint;

    #[inline]
    fn add_scalar(self, scalar: Scalar<'s>) -> Self {
        Build::add_scalar(self, scalar)
    }

    #[inline]
    fn add_composite<E, F>(self, composite: Composite<'s>, f: F) -> Result<Self, (E, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (E, Self)>,
    {
        Build::add_composite(self, composite, f)
    }

    #[inline]
    fn checkpoint(&self) -> Self::Checkpoint {
        Build::checkpoint(self)
    }

    #[inline]
    fn rollback(&mut self, checkpoint: &Self::Checkpoint) {
        Build::rollback(self, checkpoint)
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
    type Attachment = A;
    type WithAttachment<V> = Discarder<AttachmentChild<A, V>>;
    type WithoutAttachment = Discarder<AttachmentParent<A>>;
    type Checkpoint = ();

    #[inline]
    fn add_scalar(self, _: Scalar) -> Self {
        self
    }

    #[inline]
    fn add_composite<E, F>(self, _: Composite, f: F) -> Result<Self, (E, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (E, Self)>,
    {
        f(self)
    }

    #[inline]
    fn checkpoint(&self) -> Self::Checkpoint {}

    #[inline]
    fn rollback(&mut self, _: &Self::Checkpoint) {}

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

pub type ContainerInner<'s> = FlatTree<Value<'s>>;
pub type SiblingsIter<'s, 'c> = tree::SiblingsIter<'c, Value<'s>>;
pub type Node<'s, 'c> = tree::Node<'c, Value<'s>>;

// ---

#[derive(Debug, Clone)]
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
    pub const fn number(s: &'s [u8]) -> Self {
        Self::Scalar(Scalar::Number(s))
    }

    #[inline]
    pub const fn string(s: String<'s>) -> Self {
        Self::Scalar(Scalar::String(s))
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
    pub const fn field(key: String<'s>) -> Self {
        Self::Composite(Composite::Field(key))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let mut container = Container::new();
        let root = container.metaroot();
        root.add_scalar(Scalar::Bool(true))
            .add_composite::<(), _>(Composite::Array, |b| Ok(b.add_scalar(Scalar::Bool(false))))
            .map_err(|x| x.0)
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
            .add_composite::<(), _>(Composite::Array, |b| {
                let (b, attachment) = b.detach();
                assert_eq!(attachment, "attachment");
                Ok(b.add_scalar(Scalar::Bool(false)).attach("another attachment"))
            })
            .map_err(|x| x.0)
            .unwrap()
            .detach()
            .1;
        assert_eq!(container.roots().len(), 2);
        assert_eq!(attachment, "another attachment");
    }
}
