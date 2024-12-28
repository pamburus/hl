//! Data structures that allow optimization for rare heap usage.
//! Optimization can be achieved by storing part of the data in a fixed size heapless part.
//! If that capacity is not enough, the rest is stored in a heap allocated part.

pub mod indexmap;
pub mod vec;

/// Vec is a re-export of the [`vec::Vec`]`.
pub type Vec<T, const N: usize> = vec::Vec<T, N>;

/// VecIter is a re-export of the [`vec::Iter`]`.
pub type VecIter<'a, T> = vec::Iter<'a, T>;

/// VecIterMut is a re-export of the [`vec::IterMut`]`.
pub type VecIterMut<'a, T> = vec::IterMut<'a, T>;

/// IndexMap is a re-export of the [`indexmap::IndexMap`]`.
pub type IndexMap<K, V, const N: usize> = indexmap::IndexMap<K, V, N>;

/// IndexMapIter is a re-export of the [`indexmap::Iter`]`.
pub type IndexMapIter<'a, K, V> = indexmap::Iter<'a, K, V>;

/// IndexMapIterMut is a re-export of the [`indexmap::IterMut`]`.
pub type IndexMapIterMut<'a, K, V> = indexmap::IterMut<'a, K, V>;
