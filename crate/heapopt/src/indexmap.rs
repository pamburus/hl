// std imports
use std::{
    borrow::Borrow,
    hash::Hash,
    iter::{Chain, Extend},
    ops::{Index, IndexMut},
};

// ---

/// An ordered hash map that can store up to `N` elements on the stack.
#[derive(Default, Clone, Debug)]
pub struct IndexMap<K, V, const N: usize> {
    pub head: heapless::FnvIndexMap<K, V, N>,
    pub tail: indexmap::IndexMap<K, V>,
}

impl<K, V, const N: usize> IndexMap<K, V, N>
where
    K: Eq + Hash,
{
    /// Creates a new empty [`IndexMap`].
    #[inline]
    pub fn new() -> Self {
        Self {
            head: heapless::IndexMap::new(),
            tail: indexmap::IndexMap::new(),
        }
    }

    /// Returns the number of elements in the [`IndexMap`].
    #[inline]
    pub fn len(&self) -> usize {
        self.head.len() + self.tail.len()
    }

    /// Returns the element at the given index, or `None` if the index is out of bounds.
    #[inline]
    pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        if index < N {
            self.head.get_index(index)
        } else {
            self.tail.get_index(index - N)
        }
    }

    /// Returns a mutable reference to the element at the given index, or `None` if the index is out of bounds.
    #[inline]
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&K, &mut V)> {
        if index < N {
            self.head.get_index_mut(index)
        } else {
            self.tail.get_index_mut(index - N)
        }
    }

    /// Returns the index of the given key, or `None` if the key is not in the map.
    #[inline]
    pub fn get_index_of<Q>(&mut self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        if let Some(index) = self.head.get_index_of(key) {
            return Some(index);
        }

        if let Some(index) = self.tail.get_index_of(key) {
            return Some(index + N);
        }

        None
    }

    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        if let Some(value) = self.head.get(key) {
            return Some(value);
        }

        if let Some(value) = self.tail.get(key) {
            return Some(value);
        }

        None
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[inline]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        if let Some(value) = self.head.get_mut(key) {
            return Some(value);
        }

        if let Some(value) = self.tail.get_mut(key) {
            return Some(value);
        }

        None
    }

    /// Clears the map, removing all key-value pairs.
    #[inline]
    pub fn clear(&mut self) {
        self.head.clear();
        self.tail.clear();
    }

    /// Truncates the map, keeping only the first `len` elements.
    /// If `len` is greater than the length of the map, this has no effect.
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        if len <= self.head.len() {
            self.head.truncate(len);
            self.tail.clear();
        } else {
            self.tail.truncate(len - self.head.len());
        }
    }

    /// Inserts a key-value pair into the map.
    /// If the key already exists, the value is updated and the old value is returned.
    /// If the key does not exist, the key-value pair is inserted to the end and `None` is returned.
    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Hash + Eq,
    {
        match self.head.insert(key, value) {
            Ok(old) => old,
            Err((key, value)) => self.tail.insert(key, value),
        }
    }

    /// Reserves capacity for at least `additional` more elements to be inserted in the map.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        let head = N - self.head.len();
        if additional > head {
            self.tail.reserve(additional - head);
        }
    }
}

impl<'a, K, V, const N: usize> IntoIterator for &'a IndexMap<K, V, N> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.head.iter().chain(self.tail.iter())
    }
}

impl<'a, K, V, const N: usize> IntoIterator for &'a mut IndexMap<K, V, N> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.head.iter_mut().chain(self.tail.iter_mut())
    }
}

impl<'a, K, Q, V, const N: usize> Index<&'a Q> for IndexMap<K, V, N>
where
    K: Eq + Hash + Borrow<Q>,
    Q: ?Sized + Eq + Hash,
{
    type Output = V;

    #[inline]
    fn index(&self, key: &'a Q) -> &Self::Output {
        self.get(key).expect("key not found")
    }
}

impl<'a, K, Q, V, const N: usize> IndexMut<&'a Q> for IndexMap<K, V, N>
where
    K: Eq + Hash + Borrow<Q>,
    Q: ?Sized + Eq + Hash,
{
    #[inline]
    fn index_mut(&mut self, key: &'a Q) -> &mut Self::Output {
        self.get_mut(key).expect("key not found")
    }
}

impl<K, V, const N: usize> Extend<(K, V)> for IndexMap<K, V, N>
where
    K: Eq + Hash,
{
    #[inline]
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

// ---

/// An iterator over the key-value pairs of an [`IndexMap`] in the order of insertion.
pub type Iter<'a, K, V> = Chain<heapless::IndexMapIter<'a, K, V>, indexmap::map::Iter<'a, K, V>>;

/// A mutable iterator over the key-value pairs of an [`IndexMap`] in the order of insertion.
pub type IterMut<'a, K, V> = Chain<heapless::IndexMapIterMut<'a, K, V>, indexmap::map::IterMut<'a, K, V>>;

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heapopt_indexmap() {
        let mut map = IndexMap::<u32, u32, 2>::new();

        map.insert(1, 10);
        map.insert(2, 20);
        map.insert(3, 30);
        map.insert(4, 40);

        assert_eq!(map.len(), 4);

        assert_eq!(map.get_index(0), Some((&1, &10)));
        assert_eq!(map.get_index(1), Some((&2, &20)));
        assert_eq!(map.get_index(2), Some((&3, &30)));
        assert_eq!(map.get_index(3), Some((&4, &40)));
        assert_eq!(map.get_index(4), None);

        assert_eq!(map.get_index_of(&1), Some(0));
        assert_eq!(map.get_index_of(&2), Some(1));
        assert_eq!(map.get_index_of(&3), Some(2));
        assert_eq!(map.get_index_of(&4), Some(3));
        assert_eq!(map.get_index_of(&5), None);

        assert_eq!(map.get(&1), Some(&10));
        assert_eq!(map.get(&3), Some(&30));
        assert_eq!(map.get(&0), None);

        assert_eq!(map[&1], 10);
        assert_eq!(map[&3], 30);
    }
}
