// stdlib imports
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

// third-party imports
use crossbeam_queue::SegQueue;

// ---

#[allow(dead_code)]
pub trait Pool<T>: CheckOut<T> + CheckIn<T> {}

impl<T, U: CheckOut<T> + CheckIn<T>> Pool<T> for U {}

// ---

pub trait AutoPool<T>: CheckIn<T> {
    type Guard: Deref<Target = T> + DerefMut<Target = T> + AsRef<T> + AsMut<T>;

    fn auto_check_out(self: &Arc<Self>) -> Self::Guard;
}

pub struct NoPool<F = DefaultFactory> {
    f: F,
}

impl<F> NoPool<F> {
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Default for NoPool<F>
where
    F: Default,
{
    #[inline]
    fn default() -> Self {
        Self { f: Default::default() }
    }
}

impl<T, F> AutoPool<T> for NoPool<F>
where
    F: Factory<T>,
{
    type Guard = NoGuard<T>;

    #[inline]
    fn auto_check_out(self: &Arc<Self>) -> Self::Guard {
        NoGuard(self.f.new())
    }
}

impl<T, F> CheckIn<T> for NoPool<F>
where
    F: Factory<T>,
{
    #[inline]
    fn check_in(&self, _item: T) {}
}

// ---

#[allow(dead_code)]
pub trait CheckOut<T> {
    fn check_out(&self) -> T;
}

// ---

#[allow(dead_code)]
pub trait CheckIn<T> {
    fn check_in(&self, item: T);
}

// ---

pub trait Factory<T> {
    fn new(&self) -> T;
}

impl<T, F> Factory<T> for F
where
    F: Fn() -> T,
{
    #[inline]
    fn new(&self) -> T {
        self()
    }
}

// ---

pub trait Recycler<T> {
    fn recycle(&self, item: T) -> T;
}

impl<T, F> Recycler<T> for F
where
    F: Fn(T) -> T,
{
    #[inline]
    fn recycle(&self, item: T) -> T {
        self(item)
    }
}

// ---

#[derive(Default, Clone, Copy)]
pub struct DefaultFactory;

impl<T: Default> Factory<T> for DefaultFactory {
    #[inline]
    fn new(&self) -> T {
        T::default()
    }
}

// ---

pub struct RecycleAsIs;

impl<T> Recycler<T> for RecycleAsIs {
    #[inline]
    fn recycle(&self, item: T) -> T {
        item
    }
}

// ---

/// Constructs new items of type T using Factory F and recycles them using Recycler R on request.
pub struct SQPool<T, F = DefaultFactory, R = RecycleAsIs>
where
    F: Factory<T>,
    R: Recycler<T>,
{
    factory: F,
    recycler: R,
    recycled: SegQueue<T>,
}

impl<T> SQPool<T, DefaultFactory, RecycleAsIs>
where
    T: Default,
{
    /// Returns a new Pool with default factory.
    pub fn new() -> SQPool<T, DefaultFactory, RecycleAsIs> {
        SQPool {
            factory: DefaultFactory,
            recycler: RecycleAsIs,
            recycled: SegQueue::new(),
        }
    }
}

impl<T, F> SQPool<T, F, RecycleAsIs>
where
    F: Factory<T>,
{
    /// Returns a new Pool with the given factory.
    pub fn new_with_factory(factory: F) -> SQPool<T, F, RecycleAsIs> {
        SQPool {
            factory,
            recycler: RecycleAsIs,
            recycled: SegQueue::new(),
        }
    }
}

impl<T, F, R> SQPool<T, F, R>
where
    F: Factory<T>,
    R: Recycler<T>,
{
    /// Converts the Pool to a new Pool with the given factory.
    pub fn with_factory<F2: Factory<T>>(self, factory: F2) -> SQPool<T, F2, R> {
        SQPool {
            factory,
            recycler: self.recycler,
            recycled: self.recycled,
        }
    }

    /// Converts the Pool to a new Pool with the given recycle function.
    pub fn with_recycler<R2: Recycler<T>>(self, recycler: R2) -> SQPool<T, F, R2> {
        SQPool {
            factory: self.factory,
            recycler,
            recycled: self.recycled,
        }
    }
    /// Returns a new or recycled T.
    #[inline]
    pub fn check_out(&self) -> T {
        match self.recycled.pop() {
            Some(item) => item,
            None => self.factory.new(),
        }
    }
    /// Recycles the given T.
    #[inline]
    pub fn check_in(&self, item: T) {
        self.recycled.push(self.recycler.recycle(item))
    }
}

impl<T, F, R> CheckOut<T> for SQPool<T, F, R>
where
    F: Factory<T>,
    R: Recycler<T>,
{
    #[inline]
    fn check_out(&self) -> T {
        self.check_out()
    }
}

impl<T, F, R> CheckIn<T> for SQPool<T, F, R>
where
    F: Factory<T>,
    R: Recycler<T>,
{
    #[inline]
    fn check_in(&self, item: T) {
        self.check_in(item)
    }
}

impl<T, F, R> AutoPool<T> for SQPool<T, F, R>
where
    F: Factory<T>,
    R: Recycler<T>,
{
    type Guard = Guard<T, SQPool<T, F, R>>;

    #[inline]
    fn auto_check_out(self: &Arc<Self>) -> Self::Guard {
        Guard::new(self.check_out(), Arc::clone(self))
    }
}

// ---

pub struct Guard<T, P>
where
    P: CheckIn<T>,
{
    item: Option<T>,
    pool: Arc<P>,
}

impl<T, P> Guard<T, P>
where
    P: CheckIn<T>,
{
    #[inline]
    fn new(item: T, pool: Arc<P>) -> Self {
        Guard { item: Some(item), pool }
    }
}

impl<T, P> Deref for Guard<T, P>
where
    P: CheckIn<T>,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.item.as_ref().unwrap()
    }
}

impl<T, P> DerefMut for Guard<T, P>
where
    P: CheckIn<T>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.item.as_mut().unwrap()
    }
}

impl<T, P> AsRef<T> for Guard<T, P>
where
    P: CheckIn<T>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        self.item.as_ref().unwrap()
    }
}

impl<T, P> AsMut<T> for Guard<T, P>
where
    P: CheckIn<T>,
{
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.item.as_mut().unwrap()
    }
}

impl<T, P> Drop for Guard<T, P>
where
    P: CheckIn<T>,
{
    #[inline]
    fn drop(&mut self) {
        self.pool.check_in(self.item.take().unwrap())
    }
}

// ---

pub struct NoGuard<T>(T);

impl<T> Deref for NoGuard<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NoGuard<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> AsRef<T> for NoGuard<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for NoGuard<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
