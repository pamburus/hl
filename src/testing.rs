//! Testing utilities for internal use.

/// Trait that provides a method to generate a sample instance.
///
/// Types implementing this trait can create representative examples
/// of themselves for testing purposes. The `sample()` method should return
/// a stable, deterministic instance with reasonable default values.
pub trait Sample {
    /// Returns a sample instance of the implementing type.
    ///
    /// This is useful for testing where a representative example of the type is needed.
    fn sample() -> Self;
}

pub mod alloc {
    //! Counting of heap allocations made by the current thread, for checking that code doesn't allocate.

    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
    };

    thread_local! {
        static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    }

    /// The system allocator, counting allocations per thread.
    struct CountingAllocator;

    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            count();
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            count();
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            count();
            unsafe { System.realloc(ptr, layout, new_size) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    fn count() {
        _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
    }

    /// Returns the number of heap allocations, including reallocations, that `f` made on the current thread.
    pub fn count_allocations<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let before = ALLOCATIONS.with(Cell::get);
        let result = f();
        (result, ALLOCATIONS.with(Cell::get) - before)
    }
}
