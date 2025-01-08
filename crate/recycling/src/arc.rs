// std imports
use std::{
    fmt::{self, Debug, Formatter},
    mem::ManuallyDrop,
    result::Result,
    sync,
};

// third-party imports
use derive_more::{Deref, DerefMut};

// ---

#[derive(Clone, Default)]
pub struct Arc<T>(sync::Arc<ManuallyDrop<T>>);

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        Arc(sync::Arc::new(ManuallyDrop::new(data)))
    }

    pub fn into_unique(self) -> Result<UniqueArc<T>, Self> {
        UniqueArc::new(self)
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &**self.0
    }
}

impl<T: Debug> Debug for Arc<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("Arc").field(&**self).finish()
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct UniqueArc<T> {
    #[deref]
    #[deref_mut]
    data: T,
    ptr: Arc<T>,
}

impl<T> UniqueArc<T> {
    fn new(mut ptr: Arc<T>) -> Result<Self, Arc<T>> {
        match sync::Arc::get_mut(&mut ptr.0) {
            Some(inner) => Ok(Self {
                data: unsafe { ManuallyDrop::take(inner) }, // Safety: we have exclusive access to the inner value and the value is never read
                ptr,
            }),
            None => Err(ptr),
        }
    }

    pub fn share(mut self) -> Arc<T> {
        **sync::Arc::get_mut(&mut self.ptr.0).unwrap() = self.data;
        self.ptr
    }
}

impl<T: Debug> Debug for UniqueArc<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("UniqueArc").field(&self.data).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_arc() {
        let arc = Arc::new(42);
        assert!(arc.clone().into_unique().is_err());
        let mut unique = arc.into_unique().unwrap();
        assert_eq!(unique.data, 42);
        *unique = 43;
        let arc = unique.share();
        assert_eq!(*arc, 43);
    }
}
