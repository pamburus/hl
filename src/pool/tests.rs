use std::sync::Arc;

use rstest::rstest;

use super::*;

struct TestRecycler {
    capacity: usize,
}

impl Recycler for TestRecycler {
    type Item = Vec<u8>;

    fn create(&self) -> Vec<u8> {
        Vec::with_capacity(self.capacity)
    }

    fn reuse(&self, item: &mut Vec<u8>) {
        item.clear();
    }

    fn keep(&self, item: &Vec<u8>) -> bool {
        item.capacity() == self.capacity
    }
}

fn pool(capacity: usize) -> Arc<Pool<TestRecycler>> {
    Arc::new(Pool::with_recycler(TestRecycler { capacity }))
}

#[test]
fn test_take_creates_item() {
    let pool = pool(8);
    let item = pool.take();
    assert!(item.is_empty());
    assert_eq!(item.capacity(), 8);
}

#[test]
fn test_dropped_item_is_reused() {
    let pool = pool(8);
    let mut item = pool.take();
    item.extend_from_slice(b"abc");
    let ptr = item.as_ptr();
    drop(item);

    let item = pool.take();
    assert_eq!(item.as_ptr(), ptr);
    assert!(item.is_empty(), "reuse resets the item");
}

#[rstest]
#[case::kept(4, true)]
#[case::rejected(64, false)]
fn test_recycler_keep(#[case] grow_to: usize, #[case] kept: bool) {
    let pool = pool(8);
    let mut item = pool.take();
    item.reserve_exact(grow_to);
    drop(item);
    assert_eq!(pool.recycled.len(), usize::from(kept));
}

#[test]
fn test_detached_item_does_not_return() {
    let pool = pool(8);
    drop(Pooled::<TestRecycler>::detached(Vec::with_capacity(8)));
    assert_eq!(pool.recycled.len(), 0);
}

#[test]
fn test_into_inner_unlinks_item() {
    let pool = pool(8);
    let mut item = pool.take();
    item.extend_from_slice(b"abc");
    let inner = item.into_inner();
    assert_eq!(inner, b"abc");
    assert_eq!(pool.recycled.len(), 0);
}

#[test]
fn test_item_outlives_pool_handle() {
    let pool = pool(8);
    let item = pool.take();
    let weak = Arc::downgrade(&pool);
    drop(pool);
    assert!(weak.upgrade().is_some(), "the item keeps its pool alive");
    drop(item);
    assert!(weak.upgrade().is_none());
}
