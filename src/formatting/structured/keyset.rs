//! A small set of keys for checking key uniqueness in a record.

// ---

/// Number of keys kept without allocation, which covers the output keys of all predefined fields.
const INLINE_CAPACITY: usize = 8;

/// A set of keys, optimized for the few output keys of predefined fields a record has.
///
/// Keys are kept in insertion order, and a lookup is a linear scan comparing them,
/// which for a handful of keys is cheaper than hashing the key looked up.
#[derive(Default)]
pub struct KeySet<'a> {
    keys: heapopt::Vec<&'a str, INLINE_CAPACITY>,
}

impl<'a> KeySet<'a> {
    /// Inserts `key`, returning `true` if it was not in the set.
    #[inline]
    pub fn insert(&mut self, key: &'a str) -> bool {
        if self.contains(key) {
            return false;
        }
        self.keys.push(key);
        true
    }

    /// Tells whether `key` is in the set.
    #[inline]
    pub fn contains(&self, key: &str) -> bool {
        self.keys.iter().any(|&k| k == key)
    }
}

#[cfg(test)]
mod tests;
