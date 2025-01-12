use attachment::Attach;

use super::token::{Composite, Scalar};

pub trait Build: Attach
where
    Self: Sized,
{
    type Child: Build<Attachment = Self::Attachment>;
    type Error;

    fn add_scalar(self, scalar: Scalar) -> Self;
    fn add_composite<F>(self, composite: Composite, f: F) -> Result<((), Self), (Self::Error, Self)>
    where
        F: FnOnce(Self::Child) -> Result<((), Self::Child), (Self::Error, Self::Child)>;
}

// ---

pub trait Hitch<B: Build> {
    type Output;

    fn hitch(self, builder: B) -> Self::Output;
}

impl<B, R, E> Hitch<B> for Result<R, E>
where
    B: Build,
{
    type Output = Result<(R, B), (E, B)>;

    #[inline]
    fn hitch(self, builder: B) -> Self::Output {
        match self {
            Ok(r) => Ok((r, builder)),
            Err(e) => Err((e, builder)),
        }
    }
}

// ---

pub trait Unhitch<B: Build> {
    type Output;

    fn unhitch(self) -> Self::Output;
}

impl<B, R, E> Unhitch<B> for Result<(R, B), (E, B)>
where
    B: Build,
{
    type Output = (Result<R, E>, B);

    #[inline]
    fn unhitch(self) -> Self::Output {
        match self {
            Ok((r, b)) => (Ok(r), b),
            Err((e, b)) => (Err(e), b),
        }
    }
}
