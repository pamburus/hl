// std imports
use std::result::Result;

// ---

pub trait Reserve {
    fn reserve(&mut self, _additional: usize) {}
}

pub trait Push: Reserve {
    type Value;

    fn push(self, value: Self::Value) -> Self;
}

pub trait Build: Push + Sized {
    type Child: Build<Value = Self::Value>;
    type Attachment: BuildAttachment;
    type WithAttachment<V>: Build<Attachment = AttachmentChild<Self::Attachment, V>>;
    type WithoutAttachment: Build<Attachment = AttachmentParent<Self::Attachment>>;

    fn build(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Self::Child) -> Self;

    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V>;
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>);
}

impl<T: BuildE> Build for T {
    type Child = T::Child;
    type Attachment = T::Attachment;
    type WithAttachment<V> = T::WithAttachment<V>;
    type WithoutAttachment = T::WithoutAttachment;

    #[inline]
    fn build(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Self::Child) -> Self {
        unsafe { BuildE::build_e::<()>(self, value, |b| Ok(f(b))).unwrap_unchecked() }
    }

    #[inline]
    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V> {
        BuildE::attach(self, attachment)
    }

    #[inline]
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>) {
        BuildE::detach(self)
    }
}

// ---

pub trait BuildE: Push + Sized {
    type Child: BuildE<Value = Self::Value>;
    type Attachment: BuildAttachment;
    type WithAttachment<V>: BuildE<Attachment = AttachmentChild<Self::Attachment, V>>;
    type WithoutAttachment: BuildE<Attachment = AttachmentParent<Self::Attachment>>;

    fn build_e<E>(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Result<Self::Child, E>) -> Result<Self, E>;

    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V>;
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>);
}

// ---

pub trait BuildAttachment {
    type Parent: BuildAttachment;
    type Child<V>: BuildAttachment<Value = V, Parent = Self>;
    type Value;

    fn join<V>(self, value: V) -> Self::Child<V>;
    fn split(self) -> (Self::Parent, Self::Value);
}

pub type AttachmentParent<A: BuildAttachment> = <A as BuildAttachment>::Parent;
pub type AttachmentValue<A: BuildAttachment> = <A as BuildAttachment>::Value;
pub type AttachmentChild<A: BuildAttachment, V> = <A as BuildAttachment>::Child<V>;
