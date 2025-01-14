use super::token::{Composite, Scalar};

// ---

pub trait Build: Sized {
    type Error;

    fn add_scalar(self, scalar: Scalar) -> Self;
    fn add_composite<F>(self, composite: Composite, f: F) -> Result<Self, (Self::Error, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (Self::Error, Self)>;
}

// ---

pub trait Discard: Build {
    fn discard<F>(self, f: F) -> Result<Self, (Self::Error, Self)>
    where
        F: FnOnce(Discarder<Self::Error>) -> Result<Discarder<Self::Error>, (Self::Error, Discarder<Self::Error>)>,
    {
        match f(Default::default()) {
            Ok(_) => Ok(self),
            Err((e, _)) => Err((e, self)),
        }
    }
}

impl<T: Build + Sized> Discard for T {}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Discarder<E>(core::marker::PhantomData<fn(E) -> E>);

impl<E> Discarder<E> {
    #[inline]
    pub fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<E> Default for Discarder<E> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Build for Discarder<E> {
    type Error = E;

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
