use super::token::{Composite, Scalar};

// ---

pub trait Build
where
    Self: Sized,
{
    type Child: Build<Error = Self::Error>;
    type Error;

    fn add_scalar(self, scalar: Scalar) -> Self;
    fn add_composite<F>(self, composite: Composite, f: F) -> HitchResult<(), Self::Error, Self>
    where
        F: FnOnce(Self::Child) -> HitchResult<(), Self::Error, Self::Child>;
}

// ---

pub struct Discard;

impl Build for Discard {
    type Child = Discard;
    type Error = Discard;

    #[inline]
    fn add_scalar(self, _: Scalar) -> Self {
        self
    }

    #[inline]
    fn add_composite<F>(self, _: Composite, _: F) -> HitchResult<(), Self::Error, Self>
    where
        F: FnOnce(Self::Child) -> HitchResult<(), Self::Error, Self::Child>,
    {
        Ok(((), self))
    }
}

// ---

pub type HitchResult<R, E, B> = Result<(R, B), (E, B)>;

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
