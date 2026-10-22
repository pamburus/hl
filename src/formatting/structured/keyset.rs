//! A small set of keys for checking key uniqueness in a record.

// ---

/// Number of keys kept without allocation.
const INLINE_CAPACITY: usize = 32;

/// A set of keys, optimized for the few keys a record usually has.
///
/// Keys are kept with their hashes in insertion order, and their text is compared
/// only when the hashes are equal, so a lookup is a linear scan over hashes.
#[derive(Default)]
pub struct KeySet<'a> {
    keys: heapopt::Vec<(u64, &'a str), INLINE_CAPACITY>,
}

impl<'a> KeySet<'a> {
    /// Inserts `key`, returning `true` if it was not in the set.
    #[inline]
    pub fn insert(&mut self, key: &'a str) -> bool {
        self.insert_hashed(hash(key), key)
    }

    /// Tells whether `key` is in the set.
    #[inline]
    pub fn contains(&self, key: &str) -> bool {
        self.contains_hashed(hash(key), key)
    }

    #[inline]
    fn insert_hashed(&mut self, hash: u64, key: &'a str) -> bool {
        if self.contains_hashed(hash, key) {
            return false;
        }
        self.keys.push((hash, key));
        true
    }

    #[inline]
    fn contains_hashed(&self, hash: u64, key: &str) -> bool {
        self.keys.iter().any(|&(h, k)| h == hash && k == key)
    }
}

#[inline]
fn hash(key: &str) -> u64 {
    wyhash::wyhash(key.as_bytes(), 0)
}

#[cfg(test)]
mod tests;
