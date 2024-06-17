#[derive(Default)]
pub struct Vec<T, const N: usize> {
    head: heapless::Vec<T, N>,
    tail: std::vec::Vec<T>,
}

impl<T, const N: usize> Vec<T, N> {
    #[inline]
    pub fn new() -> Self {
        Self {
            head: heapless::Vec::new(),
            tail: std::vec::Vec::new(),
        }
    }

    #[inline]
    pub fn iter(&self) -> std::iter::Chain<std::slice::Iter<T>, std::slice::Iter<T>> {
        self.head.iter().chain(self.tail.iter())
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        if let Err(value) = self.head.push(value) {
            self.tail.push(value);
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if let Some(value) = self.tail.pop() {
            return Some(value);
        }
        self.head.pop()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.head.len() + self.tail.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.is_empty() && self.tail.is_empty()
    }

    #[inline]
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        self.resize_with(new_len, || value.clone());
    }

    #[inline]
    pub fn resize_default(&mut self, new_len: usize)
    where
        T: Clone + Default,
    {
        self.resize_with(new_len, || Default::default());
    }

    #[inline]
    pub fn resize_with<F>(&mut self, new_len: usize, mut f: F)
    where
        F: FnMut() -> T,
        T: Clone,
    {
        if new_len <= self.head.capacity() {
            self.head.resize(new_len, f()).ok();
            self.tail.clear();
        } else if new_len <= self.len() {
            self.tail.truncate(new_len - N);
        } else {
            let n = self.head.capacity();
            self.head.resize(n, f()).ok();
            self.tail.resize_with(new_len - n, f);
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.head.clear();
        self.tail.clear();
    }
}

impl<T, const N: usize> std::ops::Index<usize> for Vec<T, N> {
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

impl<T, const N: usize> std::ops::IndexMut<usize> for Vec<T, N> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < N {
            &mut self.head[index]
        } else {
            &mut self.tail[index - N]
        }
    }
}

impl<T, const N: usize> std::fmt::Debug for Vec<T, N>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec() {
        let mut vec = Vec::<i32, 3>::new();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());

        vec.push(1);
        vec.push(2);
        vec.push(3);
        assert_eq!(vec.len(), 3);
        assert!(!vec.is_empty());
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 3);

        vec.push(4);
        assert_eq!(vec.len(), 4);
        assert_eq!(vec[3], 4);

        vec.resize(2, 0);
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);

        vec.resize(4, 10);
        assert_eq!(vec.len(), 4);
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 10);
        assert_eq!(vec[3], 10);

        vec.resize_default(5);
        assert_eq!(vec.len(), 5);
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 10);
        assert_eq!(vec[3], 10);
        assert_eq!(vec[4], 0);

        vec.resize(4, 0);
        assert_eq!(vec.len(), 4);

        assert_eq!(vec.pop(), Some(10));
        assert_eq!(vec.pop(), Some(10));
        assert_eq!(vec.pop(), Some(2));

        vec.clear();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());

        vec.push(11);
        vec.push(23);
        vec.push(31);
        vec.push(42);
        assert_eq!(vec.len(), 4);
        assert!(!vec.is_empty());
        assert_eq!(format!("{:?}", vec), "[11, 23, 31, 42]");

        vec[1] = 24;
        vec[3] = 43;
        assert_eq!(vec[1], 24);
        assert_eq!(format!("{:?}", vec), "[11, 24, 31, 43]");
    }
}
