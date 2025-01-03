// std imports
use std::{convert::Infallible, result::Result};

// ---

pub trait Reserve {
    fn reserve(&mut self, _additional: usize) {}
}

pub trait Push: Reserve {
    type Value;

    fn push(self, value: Self::Value) -> Self;
}

// ---

pub trait Build: Push + Sized {
    type Child: Build<Value = Self::Value, Attachment = Self::Attachment>;
    type Attachment: BuildAttachment;
    type WithAttachment<V>: Build<
        Value = Self::Value,
        Attachment = AttachmentChild<Self::Attachment, V>,
        WithoutAttachment = Self,
        Child = <Self::Child as Build>::WithAttachment<V>,
    >;
    type WithoutAttachment: Build<Value = Self::Value, Attachment = AttachmentParent<Self::Attachment>>;

    fn build<R, F>(self, value: Self::Value, f: F) -> BuildOutput<F, R, Self, Self::Child>
    where
        F: FnOnce(Self::Child) -> R,
        R: BuildFnResult<F, R, Self, Self::Child>;

    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V>;
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>);
}

// ---

pub type BuildOutput<F, R, B, C> = <R as BuildFnResult<F, R, B, C>>::Output;

// ---

pub trait BuildFnResult<F, R, B, C> {
    type Error;
    type Output;

    fn into_result(self) -> Result<C, Self::Error>;
    fn finalize(result: Result<B, Self::Error>) -> Self::Output;
}

impl<F, B, C> BuildFnResult<F, C, B, C> for C
where
    F: FnOnce(C) -> C,
    B: Build,
    C: Build,
{
    type Error = Infallible;
    type Output = B;

    #[inline]
    fn into_result(self) -> Result<C, Infallible> {
        Ok(self)
    }

    #[inline]
    fn finalize(result: Result<B, Infallible>) -> B {
        result.unwrap()
    }
}

impl<F, B, C, E> BuildFnResult<F, Result<C, E>, B, C> for Result<C, E>
where
    F: FnOnce(C) -> Result<C, E>,
    B: Build,
    C: Build,
{
    type Error = E;
    type Output = Result<B, E>;

    #[inline]
    fn into_result(self) -> Result<C, E> {
        self
    }

    #[inline]
    fn finalize(result: Result<B, E>) -> Result<B, E> {
        result
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
