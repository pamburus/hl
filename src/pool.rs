//! Pools of reusable items, such as buffers, that go back to the pool they came from when dropped.

// std imports
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

// third-party imports
use crossbeam_queue::SegQueue;

// ---

/// Creates the items of a [`Pool`], prepares a returned item to be given out again, and decides which returned items are kept.
pub trait Recycler {
    /// The type of the items.
    ///
    /// Its default value is left in place of an item moved out of a [`Pooled`], so it should not allocate.
    type Item: Default;

    /// Returns a new item.
    fn create(&self) -> Self::Item;

    /// Prepares a recycled item to be given out again.
    fn reuse(&self, item: &mut Self::Item);

    /// Returns whether a returned item is kept for reuse, or dropped.
    fn keep(&self, item: &Self::Item) -> bool;
}

// ---

/// Gives out new or recycled items, see [`Pool::take`], which come back on their own when dropped.
pub struct Pool<R: Recycler> {
    recycler: R,
    recycled: SegQueue<R::Item>,
}

impl<R: Recycler> Pool<R> {
    /// Returns a new empty pool with the given recycler.
    #[inline]
    pub fn with_recycler(recycler: R) -> Self {
        Self {
            recycler,
            recycled: SegQueue::new(),
        }
    }

    /// Returns a recycled item, or a new one if there is none, linked to the pool to return to it when dropped.
    #[inline]
    pub fn take(self: &Arc<Self>) -> Pooled<R> {
        let item = match self.recycled.pop() {
            Some(mut item) => {
                self.recycler.reuse(&mut item);
                item
            }
            None => self.recycler.create(),
        };
        Pooled {
            item,
            pool: Some(self.clone()),
        }
    }

    #[inline]
    fn put(&self, item: R::Item) {
        if self.recycler.keep(&item) {
            self.recycled.push(item);
        }
    }
}

// ---

/// An item that returns to the pool it was taken from, if any, when dropped.
pub struct Pooled<R: Recycler> {
    item: R::Item,
    pool: Option<Arc<Pool<R>>>,
}

impl<R: Recycler> Pooled<R> {
    /// Returns an item that does not belong to any pool and is just dropped when it is dropped.
    #[inline]
    pub fn detached(item: R::Item) -> Self {
        Self { item, pool: None }
    }

    /// Moves the item out, which unlinks it from its pool.
    #[inline]
    pub fn into_inner(mut self) -> R::Item {
        self.pool = None;
        std::mem::take(&mut self.item)
    }
}

impl<R: Recycler> Deref for Pooled<R> {
    type Target = R::Item;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<R: Recycler> DerefMut for Pooled<R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl<R: Recycler> Drop for Pooled<R> {
    #[inline]
    fn drop(&mut self) {
        if let Some(pool) = self.pool.take() {
            pool.put(std::mem::take(&mut self.item));
        }
    }
}

// ---

#[cfg(test)]
mod tests;
