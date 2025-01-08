// std imports
use std::{mem::swap, sync::Arc};

// third-party imports
use derive_more::{Deref, DerefMut};

// ---

pub fn deconstruct<T: Default>(arc: Arc<T>) -> Option<Deconstructed<T>> {
    Deconstructed::new(arc)
}

#[derive(Default, Deref, DerefMut)]
pub struct Deconstructed<T> {
    #[deref]
    #[deref_mut]
    data: T,
    ptr: Arc<T>,
}

impl<T: Default> Deconstructed<T> {
    pub fn new(mut ptr: Arc<T>) -> Option<Self> {
        match Arc::get_mut(&mut ptr) {
            Some(inner) => {
                let mut data = T::default();
                swap(inner, &mut data);
                Some(Self { data, ptr })
            }
            None => None,
        }
    }

    pub fn construct(mut self) -> Arc<T> {
        swap(Arc::get_mut(&mut self.ptr).unwrap(), &mut self.data);
        self.ptr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_deconstruct() {
        let arc = Arc::new(42);
        assert!(deconstruct(arc.clone()).is_none());
        let mut deconstructed = deconstruct(arc).unwrap();
        assert_eq!(*deconstructed.ptr, 0);
        assert_eq!(deconstructed.data, 42);
        *deconstructed = 43;
        let arc = deconstructed.construct();
        assert_eq!(*arc, 43);
    }
}
