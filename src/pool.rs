// stdlib imports
use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    sync::Arc,
};

// third-party imports
use crossbeam_queue::SegQueue;

// workspace imports
use unique::arc::{IntoUnique, UniqueArc};

// ---

#[allow(dead_code)]
pub trait Pool<T>: CheckOut<T> + CheckIn<T> {}

impl<T, U: CheckOut<T> + CheckIn<T>> Pool<T> for U {}

// ---

pub trait Lease<T>: CheckIn<T> {
    type Leased: LeaseHold<T>;

    fn lease(self: &Arc<Self>) -> Self::Leased;

    // fn map<U, F, B>(self, forward: F, backward: B) -> MappedLease<T, Self, F, B>
    // where
    //     F: FnOnce(T) -> U,
    //     B: FnOnce(U) -> Option<T>,
    //     Self: Sized,
    // {
    //     MappedLease {
    //         lease: self,
    //         f: Some(f),
    //     }
    // }
}

pub trait LeaseHold<T>: Deref<Target = T> + DerefMut<Target = T> {
    type Pool: CheckIn<T>;

    fn into_inner(self) -> (T, Self::Pool)
    where
        Self: Sized;
}

// ---

pub trait LeaseShare<T>: LeaseHold<UniqueArc<T>> {
    type Shared: SharedLeaseHold<Self>;

    fn share(self) -> Self::Shared;
}

impl<T, H> LeaseShare<T> for H
where
    H: LeaseHold<UniqueArc<T>>,
{
    type Shared = SharedLeaseHolder<T, H::Pool>;

    fn share(self) -> Self::Shared {
        let (inner, pool) = self.into_inner();
        SharedLeaseHolder {
            ptr: ManuallyDrop::new(inner.share()),
            pool,
        }
    }
}

// ---

pub trait SharedLeaseHold<T>: Deref<Target = T> {
    type Pool: CheckIn<UniqueArc<T>>;
}

// ---

pub struct SharedLeaseHolder<T, P>
where
    P: CheckIn<UniqueArc<T>>,
{
    ptr: ManuallyDrop<Arc<T>>,
    pool: P,
}

impl<T, P> SharedLeaseHold<T> for SharedLeaseHolder<T, P>
where
    P: CheckIn<UniqueArc<T>>,
{
    type Pool = P;
}

impl<T, P> Deref for SharedLeaseHolder<T, P>
where
    P: CheckIn<UniqueArc<T>>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return &**self.ptr;
    }
}

impl<T, P> Drop for SharedLeaseHolder<T, P>
where
    P: CheckIn<UniqueArc<T>>,
{
    fn drop(&mut self) {
        // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive
        let ptr = unsafe { ManuallyDrop::take(&mut self.ptr) };
        if let Some(ptr) = ptr.into_unique() {
            self.pool.check_in(ptr);
        }
    }
}

// ---

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

impl<T, F> Lease<T> for NoPool<F>
where
    F: Factory<T>,
{
    type Leased = Granted<T>;

    #[inline]
    fn lease(self: &Arc<Self>) -> Self::Leased {
        Granted(self.f.new())
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

impl<T, F, R> Lease<T> for SQPool<T, F, R>
where
    F: Factory<T>,
    R: Recycler<T>,
{
    type Leased = Leased<T, SQPool<T, F, R>>;

    #[inline]
    fn lease(self: &Arc<Self>) -> Self::Leased {
        Leased::new(self.check_out(), Arc::clone(self))
    }
}

// ---

pub struct Leased<T, P>
where
    P: CheckIn<T>,
{
    item: ManuallyDrop<T>,
    pool: P,
}

impl<T, P> Leased<T, P>
where
    P: CheckIn<T>,
{
    #[inline]
    fn new(item: T, pool: P) -> Self {
        Leased {
            item: ManuallyDrop::new(item),
            pool,
        }
    }
}

impl<T, P> LeaseHold<T> for Leased<T, P>
where
    P: CheckIn<T>,
{
    type Pool = P;

    #[inline]
    fn into_inner(self) -> (T, Self::Pool) {
        // Safety: we do not have any special fragile logic in the destructor,
        // so we can safely deconstruct self into the inxner values.
        unsafe {
            let mut item = std::ptr::read(&self.item);
            let pool = std::ptr::read(&self.pool);
            std::mem::forget(self);

            (ManuallyDrop::take(&mut item), pool)
        }
    }
}

impl<T, P> Deref for Leased<T, P>
where
    P: CheckIn<T>,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.item
    }
}

impl<T, P> DerefMut for Leased<T, P>
where
    P: CheckIn<T>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.item
    }
}

impl<T, P> Drop for Leased<T, P>
where
    P: CheckIn<T>,
{
    #[inline]
    fn drop(&mut self) {
        self.pool.check_in(unsafe { ManuallyDrop::take(&mut self.item) })
    }
}

// ---

pub struct Granted<T>(T);

impl<T: Default> LeaseHold<T> for Granted<T> {
    type Pool = NoPool;

    #[inline]
    fn into_inner(self) -> (T, Self::Pool) {
        (self.0, NoPool::default())
    }
}

impl<T> Deref for Granted<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Granted<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> AsRef<T> for Granted<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Granted<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
