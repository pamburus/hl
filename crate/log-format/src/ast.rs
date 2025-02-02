use super::{
    token::{Composite, Scalar},
    Span,
};
use std::fmt::Display;

// ---

pub trait Build: Sized {
    // type Error: Error;

    fn add_scalar(self, scalar: Scalar) -> Self;
    fn add_composite<E, F>(self, composite: Composite, f: F) -> Result<Self, (E, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (E, Self)>;
}

// ---

pub trait Error: Display {
    fn kind(&self) -> ErrorKind;
    fn span(&self) -> Span;
}

// ---

pub trait BuilderDetach {
    type Output;
    type Builder: Build;

    fn detach(self) -> (Self::Output, Self::Builder);
}

impl<B, E> BuilderDetach for Result<B, (E, B)>
where
    B: Build,
{
    type Output = Result<(), E>;
    type Builder = B;

    fn detach(self) -> (Self::Output, Self::Builder) {
        match self {
            Ok(b) => (Ok(()), b),
            Err((e, b)) => (Err(e), b),
        }
    }
}

impl<B, T, E> BuilderDetach for Result<(T, B), (E, B)>
where
    B: Build,
{
    type Output = Result<T, E>;
    type Builder = B;

    fn detach(self) -> (Self::Output, Self::Builder) {
        match self {
            Ok((t, b)) => (Ok(t), b),
            Err((e, b)) => (Err(e), b),
        }
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum ErrorKind {
    #[default]
    InvalidToken,
    UnexpectedToken,
    UnexpectedEof,
    UnmatchedTokenPair,
    DepthLimitExceeded,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "invalid token"),
            Self::UnexpectedToken => write!(f, "unexpected token"),
            Self::UnexpectedEof => write!(f, "unexpected end of stream"),
            Self::UnmatchedTokenPair => write!(f, "no matching pair token"),
            Self::DepthLimitExceeded => write!(f, "depth limit exceeded"),
        }
    }
}

// ---

pub trait Discard: Build {
    fn discard<E, F>(self, f: F) -> Result<Self, (E, Self)>
    where
        F: FnOnce(Discarder) -> Result<Discarder, (E, Discarder)>,
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
pub struct Discarder;

impl Discarder {
    #[inline]
    pub fn new() -> Self {
        Discarder
    }
}

impl Default for Discarder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Build for Discarder {
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
}
