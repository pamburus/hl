// stdlib imports
use std::vec::Vec;

// local imports
use crate::tree::Item;

pub trait Storage {
    type Value;

    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get(&self, index: usize) -> Option<&Item<Self::Value>>;
    fn get_mut(&mut self, index: usize) -> Option<&mut Item<Self::Value>>;
    fn push(&mut self, item: Item<Self::Value>);
    fn clear(&mut self);
}

impl<V> Storage for Vec<Item<V>> {
    type Value = V;

    #[inline]
    fn len(&self) -> usize {
        Vec::len(self)
    }

    #[inline]
    fn get(&self, index: usize) -> Option<&Item<V>> {
        self.as_slice().get(index)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut Item<V>> {
        self.as_mut_slice().get_mut(index)
    }

    #[inline]
    fn push(&mut self, item: Item<V>) {
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
