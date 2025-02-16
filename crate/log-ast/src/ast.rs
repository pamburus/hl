// std imports
use std::fmt::Debug;

// workspace imports
use flat_tree::{
    FlatTree,
    tree::{self, NoAttachment},
};
use log_format::{ast, origin};

pub use log_format::{
    token::{Composite, Scalar, String},
    Format, Span,
};
pub use origin::Origin;

// ---

#[derive(Default, Debug)]
pub struct Container {
    pub inner: ContainerInner,
}

impl Container {
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

impl Container {
    #[inline]
    pub fn roots(&self) -> tree::Roots<'_, Value> {
        self.inner.roots()
    }

    #[inline]
    pub fn nodes(&self) -> tree::Nodes<'_, Value> {
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
    pub fn metaroot(&mut self) -> Builder<tree::NodeBuilder<'_, Value>> {
        Builder::new(self.inner.metaroot())
    }
}

// ---

trait InnerBuild: tree::Build<Value = Value> {}
impl<T: tree::Build<Value = Value>> InnerBuild for T {}

pub trait BuildAttachment: tree::BuildAttachment {}
impl<A: tree::BuildAttachment> BuildAttachment for A {}

pub type Children<'c> = tree::Children<'c, Value>;
pub use flat_tree::{Index, OptIndex};
pub use tree::{AttachmentChild, AttachmentParent, AttachmentValue};

pub trait Build
where
    Self: Sized,
{
    type Attachment: BuildAttachment;
    type WithAttachment<V>: Build<
            Attachment = AttachmentChild<Self::Attachment, V>,
            Checkpoint = Self::Checkpoint,
            WithoutAttachment = Self,
        >;
    type WithoutAttachment: Build<Checkpoint = Self::Checkpoint, Attachment = AttachmentParent<Self::Attachment>>;
    type Checkpoint;

    fn add_scalar(self, scalar: Scalar) -> Self;
    fn add_composite<E, F>(self, composite: Composite, f: F) -> Result<Self, (E, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (E, Self)>;

    fn checkpoint(&self) -> Self::Checkpoint;
    fn rollback(&mut self, checkpoint: &Self::Checkpoint);
    fn first_node_index(&self, checkpoint: &Self::Checkpoint) -> OptIndex;

    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V>;
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>);
}

pub trait BuildCheckpoint {
    type Builder: Build;

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

impl<T> Build for Builder<T>
where
    T: InnerBuild,
{
    type Attachment = T::Attachment;
    type WithAttachment<V> = Builder<T::WithAttachment<V>>;
    type WithoutAttachment = Builder<T::WithoutAttachment>;
    type Checkpoint = T::Checkpoint;

    #[inline]
    fn add_scalar(self, scalar: Scalar) -> Self {
        Builder::new(self.inner.push(Value::Scalar(scalar)))
    }

    #[inline]
    fn add_composite<E, F>(self, composite: Composite, f: F) -> Result<Self, (E, Self)>
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

impl<T> ast::Build for Builder<T>
where
    T: InnerBuild,
{
    type Checkpoint = T::Checkpoint;

    #[inline]
    fn add_scalar(self, scalar: Scalar) -> Self {
        Build::add_scalar(self, scalar)
    }

    #[inline]
    fn add_composite<E, F>(self, composite: Composite, f: F) -> Result<Self, (E, Self)>
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

impl<A> Build for Discarder<A>
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

pub type ContainerInner = FlatTree<Value>;
pub type SiblingsIter<'c> = tree::SiblingsIter<'c, Value>;
pub type Node<'c> = tree::Node<'c, Value>;

// ---

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Scalar(Scalar),
    Composite(Composite),
}

impl Value {
    #[inline]
    pub const fn null() -> Self {
        Self::Scalar(Scalar::Null)
    }

    #[inline]
    pub const fn bool(value: bool) -> Self {
        Self::Scalar(Scalar::Bool(value))
    }

    #[inline]
    pub const fn number(s: Span) -> Self {
        Self::Scalar(Scalar::Number(s))
    }

    #[inline]
    pub const fn string(s: String) -> Self {
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
    pub const fn field(key: String) -> Self {
        Self::Composite(Composite::Field(key))
    }
}

impl From<Scalar> for Value {
    #[inline]
    fn from(scalar: Scalar) -> Self {
        Self::Scalar(scalar)
    }
}

impl From<Composite> for Value {
    #[inline]
    fn from(composite: Composite) -> Self {
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
