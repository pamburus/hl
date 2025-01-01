// std imports
use std::result::Result;

// ---

pub trait Reserve {
    fn reserve(&mut self, _additional: usize) {}
}

pub trait Push: Reserve {
    type Value;

    fn push(self, value: Self::Value) -> Self;
}

pub trait Build: Push {
    type Child: Build<Value = Self::Value>;

    fn build(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Self::Child) -> Self;
}

// ---

pub trait BuildE: Push + Sized {
    type Child: BuildE<Value = Self::Value>;

    fn build_e<E>(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Result<Self::Child, E>) -> Result<Self, E>;
}

impl<T: BuildE> Build for T {
    type Child = T::Child;

    #[inline]
    fn build(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Self::Child) -> Self {
        unsafe { BuildE::build_e(self, value, |b| Ok::<_, ()>(f(b))).unwrap_unchecked() }
    }
}
