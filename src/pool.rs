// third-party imports
use crossbeam_queue::SegQueue;

// ---

pub trait Pool<T>: Checkout<T> + Checkin<T> {}

impl<T, U: Checkout<T> + Checkin<T>> Pool<T> for U {}

// ---

pub trait Checkout<T> {
    fn checkout(&self) -> T;
}

// ---

pub trait Checkin<T> {
    fn checkin(&self, item: T);
}

// ---

pub trait Factory<T> {
    fn new(&self) -> T;
}

impl<T, F> Factory<T> for F
where
    F: Fn() -> T,
{
    #[inline(always)]
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
    #[inline(always)]
    fn recycle(&self, item: T) -> T {
        self(item)
    }
}

// ---

pub struct DefaultFactory;

impl<T: Default> Factory<T> for DefaultFactory {
    #[inline(always)]
    fn new(&self) -> T {
        T::default()
    }
}

// ---

pub struct RecycleAsIs;

impl<T> Recycler<T> for RecycleAsIs {
    #[inline(always)]
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
    #[inline(always)]
    pub fn checkout(&self) -> T {
        match self.recycled.pop() {
            Some(item) => item,
            None => self.factory.new(),
        }
    }
    /// Recycles the given T.
    #[inline(always)]
    pub fn checkin(&self, item: T) {
        self.recycled.push(self.recycler.recycle(item))
    }
}

impl<T, F, R> Checkout<T> for SQPool<T, F, R>
where
    F: Factory<T>,
    R: Recycler<T>,
{
    #[inline(always)]
    fn checkout(&self) -> T {
        self.checkout()
    }
}

impl<T, F, R> Checkin<T> for SQPool<T, F, R>
where
    F: Factory<T>,
    R: Recycler<T>,
{
    #[inline(always)]
    fn checkin(&self, item: T) {
        self.checkin(item)
    }
}
