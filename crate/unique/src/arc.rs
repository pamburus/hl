// std imports
use std::{
    fmt::{self, Debug, Formatter},
    ops::{Deref, DerefMut},
    ptr::NonNull,
    result::Result,
    sync::Arc,
};

// ---

pub struct UniqueArc<T> {
    ptr: Arc<T>,
    data: NonNull<T>,
}

impl<T> UniqueArc<T> {
    pub fn new(value: T) -> Self {
        let ptr = Arc::new(value);
        // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive.
        unsafe { Self::from_arc_unchecked(ptr) }
    }

    pub fn share(self) -> Arc<T> {
        self.ptr
    }

    fn from_arc(mut ptr: Arc<T>) -> Result<Self, Arc<T>> {
        match Arc::get_mut(&mut ptr) {
            // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive.
            Some(_) => Ok(unsafe { Self::from_arc_unchecked(ptr) }),
            None => Err(ptr),
        }
    }

    unsafe fn from_arc_unchecked(ptr: Arc<T>) -> Self {
        debug_assert_eq!(Arc::strong_count(&ptr), 1);
        debug_assert_eq!(Arc::weak_count(&ptr), 0);

        let data = NonNull::new_unchecked(Arc::as_ptr(&ptr) as *mut T);
        Self { data, ptr }
    }
}

// Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive.
unsafe impl<T: Send> Send for UniqueArc<T> {}
unsafe impl<T: Sync> Sync for UniqueArc<T> {}

impl<T: Default> Default for UniqueArc<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> Deref for UniqueArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive.
        unsafe { self.data.as_ref() }
    }
}

impl<T> DerefMut for UniqueArc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive.
        unsafe { self.data.as_mut() }
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
        Self::from_arc(value)
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
        UniqueArc::from_arc(self).ok()
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
