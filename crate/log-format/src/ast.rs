use super::token::{Composite, Scalar};

// ---

pub trait Build
where
    Self: Sized,
{
    type Error;

    fn add_scalar(self, scalar: Scalar) -> Self;
    fn add_composite<F>(self, composite: Composite, f: F) -> Result<Self, (Self::Error, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (Self::Error, Self)>;
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Discard<E>(core::marker::PhantomData<fn(E) -> E>);

impl<E> Discard<E> {
    #[inline]
    pub fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl Default for Discard<()> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Build for Discard<E> {
    type Error = E;

    #[inline]
    fn add_scalar(self, _: Scalar) -> Self {
        self
    }

    #[inline]
    fn add_composite<F>(self, _: Composite, _: F) -> Result<Self, (Self::Error, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (Self::Error, Self)>,
    {
        Ok(self)
    }
}
