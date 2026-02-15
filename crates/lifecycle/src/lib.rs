use std::thread::{self, JoinHandle};

// ---

/// Wraps a value and calls a notification callback after dropping it.
///
/// When this `DropNotifier` is dropped, it will first drop the inner value,
/// then call the notification callback.
pub struct DropNotifier<T, F>
where
    F: FnOnce(),
{
    inner: Option<T>,
    notify: Option<F>,
}

impl<T, F> DropNotifier<T, F>
where
    F: FnOnce(),
{
    /// Creates a new `DropNotifier` wrapping `value`.
    ///
    /// When this `DropNotifier` is dropped, it will first drop `value`,
    /// then call `notify`.
    pub fn new(value: T, notify: F) -> Self {
        Self {
            inner: Some(value),
            notify: Some(notify),
        }
    }
}

impl<T, F> Drop for DropNotifier<T, F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        self.inner.take(); // drops the inner value
        if let Some(f) = self.notify.take() {
            f();
        }
    }
}

// ---

/// Drops a value in a background thread, joining on its own drop.
///
/// This is useful when dropping a value may block (e.g. `Child::wait()`),
/// and you want the drop to happen asynchronously while still ensuring
/// it completes before the owning scope exits.
pub struct AsyncDrop {
    handle: Option<JoinHandle<()>>,
}

impl AsyncDrop {
    /// Spawns a background thread that takes ownership of `value` and drops it.
    pub fn new<T: Send + 'static>(value: T) -> Self {
        let handle = thread::spawn(move || {
            drop(value);
        });
        Self { handle: Some(handle) }
    }
}

impl Drop for AsyncDrop {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().ok();
        }
    }
}
