// std imports
use std::{
    iter::{Chain, Extend},
    ops::{Index, IndexMut},
    slice,
};

// third-party imports
use derive_where::derive_where;

// ---

/// A vector that can store up to `N` elements on the stack.
#[derive(Clone, Debug)]
#[derive_where(Default)]
pub struct Vec<T, const N: usize> {
    head: heapless::Vec<T, N>,
    tail: std::vec::Vec<T>,
}

impl<T, const N: usize> Vec<T, N> {
    /// Creates a new empty vector.
    #[inline]
    pub fn new() -> Self {
        Self {
            head: heapless::Vec::new(),
            tail: std::vec::Vec::new(),
        }
    }

    /// Creates a new empty vector with the given capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            head: heapless::Vec::new(),
            tail: std::vec::Vec::with_capacity(capacity - N.min(capacity)),
        }
    }

    /// Creates a new vector from the given slice.
    #[inline]
    pub fn from_slice(other: &[T]) -> Self
    where
        T: Clone,
    {
        let mut v = Self::new();
        v.extend_from_slice(other);
        v
    }

    /// Returns the number of elements in the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.head.len() + self.tail.len()
    }

    /// Returns the total number of elements the vector can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.head.capacity() + self.tail.capacity()
    }

    /// Returns the element at the given index, or `None` if the index is out of bounds.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < N {
            self.head.get(index)
        } else {
            self.tail.get(index - N)
        }
    }

    /// Returns a mutable reference to the element at the given index, or `None` if the index is out of bounds.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < N {
            self.head.get_mut(index)
        } else {
            self.tail.get_mut(index - N)
        }
    }

    /// Returns the first element of the vector, or `None` if it is empty.
    #[inline]
    pub fn first(&self) -> Option<&T> {
        if self.head.is_empty() {
            self.tail.first()
        } else {
            self.head.first()
        }
    }

    /// Returns a mutable reference to the first element of the vector, or `None` if it is empty.
    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        if self.head.is_empty() {
            self.tail.first_mut()
        } else {
            self.head.first_mut()
        }
    }

    /// Returns the last element of the vector, or `None` if it is empty.
    #[inline]
    pub fn last(&self) -> Option<&T> {
        if self.tail.is_empty() {
            self.head.last()
        } else {
            self.tail.last()
        }
    }

    /// Returns a mutable reference to the first element of the vector, or `None` if it is empty.
    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.tail.is_empty() {
            self.head.last_mut()
        } else {
            self.tail.last_mut()
        }
    }

    /// Clears the vector, removing all elements.
    #[inline]
    pub fn clear(&mut self) {
        self.head.clear();
        self.tail.clear();
    }

    /// Truncates the vector, keeping only the first `len` elements.
    /// If `len` is greater than the length of the vector, this has no effect.
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        if len <= self.head.len() {
            self.head.truncate(len);
            self.tail.clear();
        } else {
            self.tail.truncate(len - self.head.len());
        }
    }

    /// Appends an element to the end of the vector.
    #[inline]
    pub fn push(&mut self, value: T) {
        if let Err(value) = self.head.push(value) {
            if self.tail.capacity() == 0 {
                self.tail.reserve(N);
            }
            self.tail.push(value);
        }
    }

    /// Removes the last element from the vector and returns it, or `None` if it is empty.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if let Some(value) = self.tail.pop() {
            Some(value)
        } else {
            self.head.pop()
        }
    }

    /// Returns a pair of slices containing all elements of the vector in order.
    #[inline]
    pub fn as_slices(&self) -> (&[T], &[T]) {
        (self.head.as_slice(), self.tail.as_slice())
    }

    /// Returns a pair of mutable slices containing all elements of the vector in order.
    #[inline]
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        (self.head.as_mut_slice(), self.tail.as_mut_slice())
    }

    /// Returns an iterator over the elements of the vector.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.into_iter()
    }

    /// Returns a mutable iterator over the elements of the vector.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.into_iter()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted in the vector.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        let head = N - self.head.len();
        if additional > head {
            self.tail.reserve(additional - head);
        }
    }
}

impl<T, const N: usize> Vec<T, N>
where
    T: Clone,
{
    /// Extends the vector with the elements from the given slice.
    #[inline]
    pub fn extend_from_slice(&mut self, values: &[T]) {
        let n = N - self.head.len();
        if values.len() <= n {
            self.head.extend_from_slice(values).ok();
        } else {
            self.head.extend_from_slice(&values[..n]).ok();
            self.tail.extend_from_slice(&values[n..]);
        }
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a Vec<T, N> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.head.iter().chain(self.tail.iter())
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut Vec<T, N> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.head.iter_mut().chain(self.tail.iter_mut())
    }
}

impl<T, const N: usize> Index<usize> for Vec<T, N> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        if index < N {
            &self.head[index]
        } else {
            &self.tail[index - N]
        }
    }
}

impl<T, const N: usize> IndexMut<usize> for Vec<T, N> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < N {
            &mut self.head[index]
        } else {
            &mut self.tail[index - N]
        }
    }
}

impl<T, const N: usize> Extend<T> for Vec<T, N>
where
    T: Clone,
{
    #[inline]
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        let mut iter = iter.into_iter();
        let head = N - self.head.len();
        if head > 0 {
            self.head.extend(iter.by_ref().take(head));
        }
        self.tail.extend(iter);
    }
}

// ---

/// An iterator over the elements of a vector.
pub type Iter<'a, T> = Chain<slice::Iter<'a, T>, slice::Iter<'a, T>>;

/// A mutable iterator over the elements of a vector.
pub type IterMut<'a, T> = Chain<slice::IterMut<'a, T>, slice::IterMut<'a, T>>;

// ---

#[cfg(test)]
mod tests {
    use super::*;

    // third-party imports
    use more_asserts::*;

    #[test]
    fn test_vec() {
        let mut vec = Vec::<i32, 3>::new();
        assert_eq!(vec.len(), 0);

        vec.push(1);
        vec.push(2);
        vec.push(3);
        assert_eq!(vec.len(), 3);

        vec.push(4);
        assert_eq!(vec.len(), 4);

        vec.clear();
        assert_eq!(vec.len(), 0);

        vec.push(1);
        vec.push(2);
        vec.push(3);
        vec.truncate(2);
        assert_eq!(vec.len(), 2);

        vec.push(3);
        vec.push(4);
        vec.push(5);
        vec.truncate(4);
        assert_eq!(vec.len(), 4);

        assert_eq!(vec.get(0), Some(&1));
        assert_eq!(vec.get(1), Some(&2));
        assert_eq!(vec.get(3), Some(&4));

        assert_eq!(vec.get_mut(0), Some(&mut 1));
        assert_eq!(vec.get_mut(1), Some(&mut 2));
        assert_eq!(vec.get_mut(3), Some(&mut 4));

        let mut vec = Vec::<i32, 3>::from_slice(&[1, 2, 3, 4]);
        assert_eq!(vec.len(), 4);

        vec.clear();
        vec.extend_from_slice(&[1, 2, 3, 4, 5]);
        assert_eq!(vec.len(), 5);

        assert_eq!(vec.as_slices().0, &[1, 2, 3]);
        assert_eq!(vec.as_slices().1, &[4, 5]);
        assert_eq!(vec.as_mut_slices().0, &mut [1, 2, 3]);
        assert_eq!(vec.as_mut_slices().1, &mut [4, 5]);

        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 3);
        assert_eq!(vec[3], 4);

        vec[1] = 6;
        assert_eq!(vec[1], 6);

        vec[3] = 7;
        assert_eq!(vec[3], 7);

        let mut iter = vec.iter();
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&6));

        let mut iter = vec.iter_mut();
        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next(), Some(&mut 6));

        assert_eq!(vec.first(), Some(&1));
        assert_eq!(vec.first_mut(), Some(&mut 1));
        assert_eq!(vec.last(), Some(&5));
        assert_eq!(vec.last_mut(), Some(&mut 5));

        assert_eq!(vec.pop(), Some(5));
        assert_eq!(vec.pop(), Some(7));
        assert_eq!(vec.pop(), Some(3));
        assert_eq!(vec.pop(), Some(6));

        assert_eq!(vec.first(), Some(&1));
        assert_eq!(vec.first_mut(), Some(&mut 1));
        assert_eq!(vec.last(), Some(&1));
        assert_eq!(vec.last_mut(), Some(&mut 1));

        assert_eq!(vec.pop(), Some(1));
        assert_eq!(vec.pop(), None);

        assert_eq!(vec.first(), None);
        assert_eq!(vec.first_mut(), None);
        assert_eq!(vec.last(), None);
        assert_eq!(vec.last_mut(), None);

        assert_eq!(Vec::<i32, 2>::with_capacity(3).capacity(), 3);
        assert_eq!(Vec::<i32, 2>::with_capacity(1).capacity(), 2);
        assert_eq!(Vec::<i32, 2>::with_capacity(0).capacity(), 2);

        let mut vec = Vec::<i32, 3>::new();

        vec.reserve(2);
        assert_eq!(vec.capacity(), 3);

        vec.reserve(3);
        assert_eq!(vec.capacity(), 3);

        vec.reserve(4);
        let cap = vec.capacity();
        assert_ge!(cap, 4);

        vec.extend([1, 2].iter().cloned());
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.capacity(), cap);
        assert_eq!(vec.as_slices().0, &[1, 2]);

        vec.extend([3, 4].iter().cloned());
        assert_eq!(vec.len(), 4);
        assert_eq!(vec.capacity(), cap);
        assert_eq!(vec.as_slices().0, &[1, 2, 3]);
        assert_eq!(vec.as_slices().1, &[4]);

        vec.extend([5, 6].iter().cloned());
        assert_eq!(vec.len(), 6);
        assert_ge!(vec.capacity(), 6);
        assert_eq!(vec.as_slices().0, &[1, 2, 3]);
        assert_eq!(vec.as_slices().1, &[4, 5, 6]);
    }
}
