// stdlib imports
use std::vec::Vec;

// local imports
use crate::tree::Item;

pub trait StorageType<V> {
    type Storage: Storage<Item<V>>;
}

pub struct VecStorage;

impl<V> StorageType<V> for VecStorage {
    type Storage = Vec<Item<V>>;
}

pub trait Storage<T> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get(&self, index: usize) -> Option<&T>;
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;
    fn push(&mut self, item: T);
    fn clear(&mut self);
}

impl<T> Storage<T> for Vec<T> {
    #[inline]
    fn len(&self) -> usize {
        Vec::len(self)
    }

    #[inline]
    fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.as_mut_slice().get_mut(index)
    }

    #[inline]
    fn push(&mut self, item: T) {
        Vec::push(self, item)
    }

    #[inline]
    fn clear(&mut self) {
        Vec::clear(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage() {
        let mut storage = Vec::new();
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
        storage.push(1);
        assert_eq!(storage.len(), 1);
        assert!(!storage.is_empty());
        assert_eq!(storage.get(0), Some(&1));
        assert_eq!(storage.get_mut(0), Some(&mut 1));
        storage.clear();
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }
}