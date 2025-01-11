// stdlib imports
use std::{
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    sync::Arc,
};

// third-party imports
use crossbeam_queue::SegQueue;

// workspace imports
use unique::arc::{IntoUnique, UniqueArc};

// ---

pub trait Pool {
    type Item;

    fn check_out(&self) -> Self::Item;
    fn check_in(&self, item: Self::Item);
}

impl<P> Pool for Arc<P>
where
    P: Pool,
{
    type Item = P::Item;

    #[inline]
    fn check_in(&self, item: Self::Item) {
        self.as_ref().check_in(item)
    }

    #[inline]
    fn check_out(&self) -> Self::Item {
        self.as_ref().check_out()
    }
}

// ---

pub trait Lease {
    type Payload;
    type Item;
    type Pool: Pool<Item = Self::Item>;
    type Leased: LeaseHold<Payload = Self::Payload>;

    fn lease(&self) -> Self::Leased;
}

pub trait LeaseHold: DerefMut {
    type Payload;
    type Item;
    type Pool: Pool<Item = Self::Item>;

    fn into_inner(self) -> (Self::Item, Self::Pool)
    where
        Self: Sized;
}

// ---

pub trait LeaseShare: LeaseHold {
    type Shared: SharedLeaseHold<Payload = Self::Payload>;

    fn share(self) -> Self::Shared;
}

// ---

pub trait SharedLeaseHold: Deref + Clone {
    type Payload;
    type Item: Deref<Target = Self::Payload>;
    type Pool: Pool<Item = Self::Item>;
}

// ---

pub struct SharedLeaseHolder<T, P>
where
    P: Pool<Item = UniqueArc<T>> + Clone,
{
    ptr: ManuallyDrop<Arc<T>>,
    pool: P,
}

impl<T, P> Clone for SharedLeaseHolder<T, P>
where
    P: Pool<Item = UniqueArc<T>> + Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            ptr: ManuallyDrop::new(Arc::clone(&self.ptr)),
            pool: self.pool.clone(),
        }
    }
}

impl<T, P> SharedLeaseHold for SharedLeaseHolder<T, P>
where
    P: Pool<Item = UniqueArc<T>> + Clone,
{
    type Payload = T;
    type Item = UniqueArc<T>;
    type Pool = P;
}

impl<T, P> Deref for SharedLeaseHolder<T, P>
where
    P: Pool<Item = UniqueArc<T>> + Clone,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        return &**self.ptr;
    }
}

impl<T, P> Drop for SharedLeaseHolder<T, P>
where
    P: Pool<Item = UniqueArc<T>> + Clone,
{
    #[inline]
    fn drop(&mut self) {
        // Safety: we have exclusive access to the inner value and the pointer is valid as long as the Arc is alive
        let ptr = unsafe { ManuallyDrop::take(&mut self.ptr) };
        if let Some(ptr) = ptr.into_unique() {
            self.pool.check_in(ptr);
        }
    }
}

// ---

pub trait AsPayload<T: ?Sized> {
    fn as_payload(&self) -> &T;
}

pub trait AsPayloadMut<T: ?Sized> {
    fn as_payload_mut(&mut self) -> &mut T;
}

// ---

pub trait Factory {
    type Item;

    fn new(&self) -> Self::Item;
}

impl<T, F> Factory for F
where
    F: Fn() -> T,
{
    type Item = T;

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
pub struct DefaultFactory<T>(PhantomData<T>);

impl<T: Default> Factory for DefaultFactory<T> {
    type Item = T;

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
pub struct SQPool<T, F = DefaultFactory<T>, R = RecycleAsIs>
where
    F: Factory<Item = T>,
    R: Recycler<T>,
{
    factory: F,
    recycler: R,
    recycled: SegQueue<T>,
}

impl<T> SQPool<T, DefaultFactory<T>, RecycleAsIs>
where
    T: Default,
{
    /// Returns a new Pool with default factory.
    pub fn new() -> SQPool<T, DefaultFactory<T>, RecycleAsIs> {
        SQPool {
            factory: DefaultFactory(PhantomData),
            recycler: RecycleAsIs,
            recycled: SegQueue::new(),
        }
    }
}

impl<T, F> SQPool<T, F, RecycleAsIs>
where
    F: Factory<Item = T>,
{
    /// Returns a new Pool with the given factory.
    #[inline]
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
    F: Factory<Item = T>,
    R: Recycler<T>,
{
    /// Converts the Pool to a new Pool with the given factory.
    #[inline]
    pub fn with_factory<F2: Factory<Item = T>>(self, factory: F2) -> SQPool<T, F2, R> {
        SQPool {
            factory,
            recycler: self.recycler,
            recycled: self.recycled,
        }
    }

    /// Converts the Pool to a new Pool with the given recycle function.
    #[inline]
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

impl<T, F, R> Pool for SQPool<T, F, R>
where
    F: Factory<Item = T>,
    R: Recycler<T>,
{
    type Item = T;

    #[inline]
    fn check_out(&self) -> T {
        self.check_out()
    }

    #[inline]
    fn check_in(&self, item: T) {
        self.check_in(item)
    }
}

impl<T, F, R> Lease for Arc<SQPool<T, F, R>>
where
    F: Factory<Item = T>,
    R: Recycler<T>,
{
    type Payload = T;
    type Item = T;
    type Pool = Self;
    type Leased = Leased<T, Self>;

    #[inline]
    fn lease(&self) -> Self::Leased {
        Leased::new(self.check_out(), self.clone())
    }
}

// ---

pub struct ArcSQPool<T, F = DefaultFactory<UniqueArc<T>>, R = RecycleAsIs>(SQPool<UniqueArc<T>, F, R>)
where
    F: Factory<Item = UniqueArc<T>>,
    R: Recycler<UniqueArc<T>>;

impl<T> ArcSQPool<T>
where
    T: Default,
{
    pub fn new() -> Self {
        Self(SQPool::new())
    }
}

impl<T, F> ArcSQPool<T, F, RecycleAsIs>
where
    F: Factory<Item = UniqueArc<T>>,
{
    /// Returns a new Pool with the given factory.
    #[inline]
    pub fn new_with_factory(factory: F) -> ArcSQPool<T, F, RecycleAsIs> {
        Self(SQPool::new_with_factory(factory))
    }
}

impl<T, F, R> ArcSQPool<T, F, R>
where
    F: Factory<Item = UniqueArc<T>>,
    R: Recycler<UniqueArc<T>>,
{
    /// Converts the Pool to a new Pool with the given factory.
    #[inline]
    pub fn with_factory<F2: Factory<Item = UniqueArc<T>>>(self, factory: F2) -> ArcSQPool<T, F2, R> {
        ArcSQPool(self.0.with_factory(factory))
    }

    /// Converts the Pool to a new Pool with the given recycle function.
    #[inline]
    pub fn with_recycler<R2: Recycler<UniqueArc<T>>>(self, recycler: R2) -> ArcSQPool<T, F, R2> {
        ArcSQPool(self.0.with_recycler(recycler))
    }

    /// Returns a new or recycled item.
    #[inline]
    pub fn check_out(&self) -> UniqueArc<T> {
        self.0.check_out()
    }

    /// Recycles the given item.
    #[inline]
    pub fn check_in(&self, item: UniqueArc<T>) {
        self.0.check_in(item);
    }
}

impl<T, F, R> Pool for ArcSQPool<T, F, R>
where
    F: Factory<Item = UniqueArc<T>>,
    R: Recycler<UniqueArc<T>>,
{
    type Item = UniqueArc<T>;

    #[inline]
    fn check_out(&self) -> UniqueArc<T> {
        self.check_out()
    }

    #[inline]
    fn check_in(&self, item: UniqueArc<T>) {
        self.check_in(item)
    }
}

impl<T, F, R> Lease for Arc<ArcSQPool<T, F, R>>
where
    F: Factory<Item = UniqueArc<T>>,
    R: Recycler<UniqueArc<T>>,
{
    type Payload = T;
    type Item = UniqueArc<T>;
    type Pool = ArcSQPool<T, F, R>;
    type Leased = LeasedUnique<T, Arc<ArcSQPool<T, F, R>>>;

    #[inline]
    fn lease(&self) -> Self::Leased {
        LeasedUnique::new(self.check_out(), Arc::clone(self))
    }
}

// ---

pub struct Leased<T, P>
where
    P: Pool<Item = T>,
{
    item: ManuallyDrop<T>,
    pool: P,
}

impl<T, P> Leased<T, P>
where
    P: Pool<Item = T>,
{
    #[inline]
    fn new(item: T, pool: P) -> Self {
        Leased {
            item: ManuallyDrop::new(item),
            pool,
        }
    }
}

impl<T, P> AsPayload<T> for Leased<T, P>
where
    P: Pool<Item = T>,
{
    #[inline]
    fn as_payload(&self) -> &T {
        &*self.item
    }
}

impl<T, P> AsPayloadMut<T> for Leased<T, P>
where
    P: Pool<Item = T>,
{
    #[inline]
    fn as_payload_mut(&mut self) -> &mut T {
        &mut *self.item
    }
}

impl<T, P> LeaseHold for Leased<T, P>
where
    P: Pool<Item = T>,
{
    type Payload = T;
    type Item = T;
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
    P: Pool<Item = T>,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.item
    }
}

impl<T, P> DerefMut for Leased<T, P>
where
    P: Pool<Item = T>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.item
    }
}

impl<T, P> Drop for Leased<T, P>
where
    P: Pool<Item = T>,
{
    #[inline]
    fn drop(&mut self) {
        self.pool.check_in(unsafe { ManuallyDrop::take(&mut self.item) })
    }
}

// ---

pub struct LeasedUnique<T, P>(Leased<UniqueArc<T>, P>)
where
    P: Pool<Item = UniqueArc<T>>;

impl<T, P> LeasedUnique<T, P>
where
    P: Pool<Item = UniqueArc<T>>,
{
    #[inline]
    fn new(item: UniqueArc<T>, pool: P) -> Self {
        Self(Leased::new(item, pool))
    }
}

impl<T, P> AsPayload<T> for LeasedUnique<T, P>
where
    P: Pool<Item = UniqueArc<T>>,
{
    #[inline]
    fn as_payload(&self) -> &T {
        &*self.0
    }
}

impl<T, P> AsPayloadMut<T> for LeasedUnique<T, P>
where
    P: Pool<Item = UniqueArc<T>>,
{
    #[inline]
    fn as_payload_mut(&mut self) -> &mut T {
        &mut *self.0
    }
}

impl<T, P> LeaseHold for LeasedUnique<T, P>
where
    P: Pool<Item = UniqueArc<T>>,
{
    type Payload = T;
    type Item = UniqueArc<T>;
    type Pool = P;

    #[inline]
    fn into_inner(self) -> (Self::Item, Self::Pool) {
        self.0.into_inner()
    }
}

impl<T, P> LeaseShare for LeasedUnique<T, P>
where
    P: Pool<Item = UniqueArc<T>> + Clone,
{
    type Shared = SharedLeaseHolder<T, P>;

    #[inline]
    fn share(self) -> Self::Shared {
        let (inner, pool) = self.into_inner();
        SharedLeaseHolder {
            ptr: ManuallyDrop::new(inner.share()),
            pool,
        }
    }
}

impl<T, P> Deref for LeasedUnique<T, P>
where
    P: Pool<Item = UniqueArc<T>>,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.as_payload()
    }
}

impl<T, P> DerefMut for LeasedUnique<T, P>
where
    P: Pool<Item = UniqueArc<T>>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.as_payload_mut()
    }
}

// ---

#[cfg(test)]
mod tests {
    #[test]
    fn test_pool() {
        use super::*;

        let pool = Arc::new(SQPool::new_with_factory(|| 42));
        let mut leased = pool.lease();
        assert_eq!(*leased, 42);
        *leased = 43;
        assert_eq!(*leased, 43);
        drop(leased);

        let leased = pool.lease();
        assert_eq!(*leased, 43);
        let mut leased = pool.lease();
        assert_eq!(*leased, 42);
        *leased = 44;
        assert_eq!(*leased, 44);

        let pool = Arc::new(ArcSQPool::new_with_factory(|| UniqueArc::new(42)));

        let mut leased = pool.lease();
        assert_eq!(*leased, 42);
        *leased = 43;
        assert_eq!(*leased, 43);

        let shared = leased.share();
        assert_eq!(*shared, 43);

        let cloned = shared.clone();
        assert_eq!(*cloned, 43);
    }
}
