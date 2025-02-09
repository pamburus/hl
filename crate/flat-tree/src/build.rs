// std imports
use std::result::Result;

// local imports
use super::OptIndex;

// ---

pub trait Reserve {
    fn reserve(&mut self, _additional: usize) {}
}

pub trait Push: Reserve {
    type Value;
    type Checkpoint;

    fn push(self, value: Self::Value) -> Self;

    fn checkpoint(&self) -> Self::Checkpoint;
    fn rollback(&mut self, checkpoint: &Self::Checkpoint);
    fn first_node_index(&self, checkpoint: &Self::Checkpoint) -> OptIndex;
}

// ---

pub trait Build: Push + Sized {
    type Attachment: BuildAttachment;
    type WithAttachment<V>: Build<
        Value = Self::Value,
        Checkpoint = Self::Checkpoint,
        Attachment = AttachmentChild<Self::Attachment, V>,
        WithoutAttachment = Self,
    >;
    type WithoutAttachment: Build<
        Value = Self::Value,
        Checkpoint = Self::Checkpoint,
        Attachment = AttachmentParent<Self::Attachment>,
    >;

    fn build<R, F>(self, value: Self::Value, f: F) -> BuildOutput<F, R, Self>
    where
        F: FnOnce(Self) -> R,
        R: BuildFnResult<F, R, Self>;

    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V>;
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>);
}

// ---

pub type BuildOutput<F, R, B> = <R as BuildFnResult<F, R, B>>::Output;

// ---

pub trait BuildFnResult<F, R, B> {
    type Output;

    fn transform<MF: FnOnce(B) -> B>(self, map: MF) -> Self::Output;
}

impl<F, B> BuildFnResult<F, B, B> for B
where
    F: FnOnce(B) -> B,
    B: Build,
{
    type Output = B;

    #[inline]
    fn transform<MF: FnOnce(B) -> B>(self, map: MF) -> Self::Output {
        map(self)
    }
}

impl<F, B, E> BuildFnResult<F, Result<B, E>, B> for Result<B, E>
where
    F: FnOnce(B) -> Result<B, E>,
    B: Build,
{
    type Output = Result<B, E>;

    #[inline]
    fn transform<MF: FnOnce(B) -> B>(self, map: MF) -> Self::Output {
        self.map(map)
    }
}

impl<F, B, R> BuildFnResult<F, (B, R), B> for (B, R)
where
    F: FnOnce(B) -> (B, R),
    B: Build,
{
    type Output = (B, R);

    #[inline]
    fn transform<MF: FnOnce(B) -> B>(self, map: MF) -> Self::Output {
        (map(self.0), self.1)
    }
}

// ---

pub trait BuildAttachment {
    type Parent: BuildAttachment;
    type Child<V>: BuildAttachment<Value = V, Parent = Self>;
    type Value;

    fn join<V>(self, value: V) -> Self::Child<V>;
    fn split(self) -> (Self::Parent, Self::Value);
}

pub type AttachmentParent<A> = <A as BuildAttachment>::Parent;
pub type AttachmentValue<A> = <A as BuildAttachment>::Value;
pub type AttachmentChild<A, V> = <A as BuildAttachment>::Child<V>;
