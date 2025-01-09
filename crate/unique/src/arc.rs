// std imports
use std::{
    fmt::{self, Debug, Formatter},
    ops::{Deref, DerefMut},
    result::Result,
    sync::Arc,
};

// ---

pub struct UniqueArc<T> {
    ptr: Arc<T>,
    data: *mut T,
}

impl<T> UniqueArc<T> {
    fn new(mut ptr: Arc<T>) -> Result<Self, Arc<T>> {
        match Arc::get_mut(&mut ptr) {
            Some(_) => Ok(Self {
                // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive
                data: unsafe { std::mem::transmute(Arc::as_ptr(&ptr)) },
                ptr,
            }),
            None => Err(ptr),
        }
    }

    pub fn share(self) -> Arc<T> {
        self.ptr
    }
}

impl<T: Default> Default for UniqueArc<T> {
    fn default() -> Self {
        let ptr = Arc::new(Default::default());
        // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive
        let data = unsafe { std::mem::transmute(Arc::as_ptr(&ptr)) };
        Self { ptr, data }
    }
}

impl<T> Deref for UniqueArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive
        unsafe { &*self.data }
    }
}

impl<T> DerefMut for UniqueArc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive
        unsafe { &mut *self.data }
    }
}

impl<T: Debug> Debug for UniqueArc<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("UniqueArc").field(&self.data).finish()
    }
}

impl<T> TryFrom<Arc<T>> for UniqueArc<T> {
    type Error = Arc<T>;

    fn try_from(value: Arc<T>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<T> From<UniqueArc<T>> for Arc<T> {
    fn from(value: UniqueArc<T>) -> Self {
        value.share()
    }
}

pub trait IntoUnique {
    type Item;

    fn into_unique(self) -> Option<UniqueArc<Self::Item>>
    where
        Self: Sized;
}

impl<T> IntoUnique for Arc<T> {
    type Item = T;

    fn into_unique(self) -> Option<UniqueArc<T>> {
        UniqueArc::new(self).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_arc() {
        let arc = Arc::new(42);
        assert!(arc.clone().into_unique().is_none());
        let mut unique = arc.into_unique().unwrap();
        assert_eq!(*unique, 42);
        *unique = 43;
        let arc = unique.share();
        assert_eq!(*arc, 43);
    }
}
