// std imports
use std::result::Result;

// third-party imports
use derive_more::{Deref, DerefMut};

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

    fn build(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Self::Child) -> Self {
        self.build_s(value, (), |b| BuilderAndState::new(f(b.into_builder()), ()))
            .into_builder()
    }

    fn build_s<S>(
        self,
        value: Self::Value,
        state: S,
        f: impl FnOnce(BuilderAndState<Self::Child, S>) -> BuilderAndState<Self::Child, S>,
    ) -> BuilderAndState<Self, S>;
}

impl<T: BuildE> Build for T {
    type Child = T::Child;

    #[inline]
    fn build_s<S>(
        self,
        value: Self::Value,
        state: S,
        f: impl FnOnce(BuilderAndState<Self::Child, S>) -> BuilderAndState<Self::Child, S>,
    ) -> BuilderAndState<Self, S> {
        unsafe { BuildE::build_es::<(), _>(self, value, state, |b| Ok(f(b))).unwrap_unchecked() }
    }
}

// ---

pub trait BuildE: Push + Sized {
    type Child: BuildE<Value = Self::Value>;

    fn build_e<E>(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Result<Self::Child, E>) -> Result<Self, E> {
        Ok(self
            .build_es::<E, _>(value, (), |b| f(b.into_builder()).map(|r| BuilderAndState::new(r, ())))?
            .into_builder())
    }

    fn build_es<E, S>(
        self,
        value: Self::Value,
        state: S,
        f: impl FnOnce(BuilderAndState<Self::Child, S>) -> Result<BuilderAndState<Self::Child, S>, E>,
    ) -> Result<BuilderAndState<Self, S>, E>;
}

// ---

#[derive(Deref, DerefMut)]
pub struct BuilderAndState<B, S> {
    #[deref]
    #[deref_mut]
    builder: B,
    state: S,
}

impl<B, S> BuilderAndState<B, S> {
    #[inline]
    pub fn new(builder: B, state: S) -> Self {
        Self { builder, state }
    }

    #[inline]
    pub fn split(self) -> (B, S) {
        (self.builder, self.state)
    }

    #[inline]
    pub fn into_builder(self) -> B {
        self.builder
    }

    #[inline]
    pub fn into_state(self) -> S {
        self.state
    }
}
