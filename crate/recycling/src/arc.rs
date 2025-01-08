// std imports
use std::{mem::ManuallyDrop, sync};

// third-party imports
use derive_more::{Deref, DerefMut};

// ---

#[derive(Clone, Default)]
pub struct Arc<T>(sync::Arc<ManuallyDrop<T>>);

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        Arc(sync::Arc::new(ManuallyDrop::new(data)))
    }

    pub fn deconstruct(self) -> Option<Deconstructed<T>> {
        Deconstructed::new(self)
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &**self.0
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct Deconstructed<T> {
    #[deref]
    #[deref_mut]
    data: T,
    ptr: Arc<T>,
}

impl<T> Deconstructed<T> {
    fn new(mut ptr: Arc<T>) -> Option<Self> {
        match sync::Arc::get_mut(&mut ptr.0) {
            Some(inner) => Some(Self {
                data: unsafe { ManuallyDrop::take(inner) },
                ptr,
            }),
            None => None,
        }
    }

    pub fn construct(mut self) -> Arc<T> {
        **sync::Arc::get_mut(&mut self.ptr.0).unwrap() = self.data;
        self.ptr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deconstruct() {
        let arc = Arc::new(42);
        assert!(arc.clone().deconstruct().is_none());
        let mut deconstructed = arc.deconstruct().unwrap();
        assert_eq!(deconstructed.data, 42);
        *deconstructed = 43;
        let arc = deconstructed.construct();
        assert_eq!(*arc, 43);
    }
}
