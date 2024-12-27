// stdlib imports
use std::vec::Vec;

// local imports
use crate::{storage::Storage, tree::Item};

pub trait Domain {
    type Value;
    type Storage: Storage<Item<Self::Value>>;
}

impl<T> Domain for Vec<T> {
    type Value = T;
    type Storage = Vec<Item<T>>;
}

pub struct DefaultDomain<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Domain for DefaultDomain<T> {
    type Value = T;
    type Storage = Vec<Item<T>>;
}
