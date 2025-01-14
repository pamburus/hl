use super::token::{Composite, Scalar};

// ---

pub trait Build
where
    Self: Sized,
{
    type Error;
    type Discard: Build<Error = Self::Error> + Default;

    fn add_scalar(self, scalar: Scalar) -> Self;

    fn add_composite<F>(self, composite: Composite, f: F) -> Result<Self, (Self::Error, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (Self::Error, Self)>;

    fn discard<F>(self, f: F) -> Result<Self, (Self::Error, Self)>
    where
        F: FnOnce(Self::Discard) -> Result<Self::Discard, (Self::Error, Self::Discard)>,
    {
        match f(Default::default()) {
            Ok(_) => Ok(self),
            Err((e, _)) => Err((e, self)),
        }
    }
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

impl<E> Default for Discard<E> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Build for Discard<E> {
    type Error = E;
    type Discard = Self;

    #[inline]
    fn add_scalar(self, _: Scalar) -> Self {
        self
    }

    #[inline]
    fn add_composite<F>(self, _: Composite, f: F) -> Result<Self, (Self::Error, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (Self::Error, Self)>,
    {
        f(self)
    }
}
