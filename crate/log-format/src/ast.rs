use super::{
    token::{Composite, Scalar},
    Span,
};
use std::fmt::Display;

// ---

pub trait Build<'s>: Sized {
    type Checkpoint;

    fn add_scalar(self, scalar: Scalar<'s>) -> Self;
    fn add_composite<E, F>(self, composite: Composite<'s>, f: F) -> Result<Self, (E, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (E, Self)>;

    fn checkpoint(&self) -> Self::Checkpoint;
    fn rollback(&mut self, checkpoint: &Self::Checkpoint);
}

// ---

pub trait Error: Display {
    fn kind(&self) -> ErrorKind;
    fn span(&self) -> Span;
}

// ---

pub trait BuilderDetach<'s> {
    type Output;
    type Builder: Build<'s>;

    fn detach(self) -> (Self::Output, Self::Builder);
}

impl<'s, B, E> BuilderDetach<'s> for Result<B, (E, B)>
where
    B: Build<'s>,
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

impl<'s, B, T, E> BuilderDetach<'s> for Result<(T, B), (E, B)>
where
    B: Build<'s>,
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

pub trait Discard<'s>: Build<'s> {
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

impl<'s, T: Build<'s> + Sized> Discard<'s> for T {}

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

impl Build<'_> for Discarder {
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
}
