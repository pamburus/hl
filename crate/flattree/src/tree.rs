// std imports
use std::marker::PhantomData;

// local imports
use crate::storage::Storage;

pub struct FlatTree<T, S = Vec<Item<T>>>
where
    S: Storage<Value = T>,
{
    s: S,
    _marker: PhantomData<T>,
}

impl<T, S> FlatTree<T, S>
where
    S: Storage<Value = T>,
{
    pub fn build() -> FlatTreeBuilder<S> {
        FlatTreeBuilder::new(Default::default())
    }
}

impl<T, S> FlatTree<T, S>
where
    S: Storage<Value = T>,
{
    pub fn build_with(storage: S) -> FlatTreeBuilder<S> {
        FlatTreeBuilder::new(storage)
    }
}

pub struct FlatTreeBuilder<S> {
    storage: S,
}

impl<S> FlatTreeBuilder<S>
where
    S: Storage,
{
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    pub fn root<'b>(&'b mut self) -> NodeBuilder<'b, Self> {
        NodeBuilder {
            b: self,
            parent: None,
            ld: 0,
        }
    }

    pub fn done(self) -> FlatTree<S::Value, S> {
        FlatTree {
            s: self.storage,
            _marker: PhantomData,
        }
    }

    pub fn add(mut self, value: S::Value) -> Self {
        self.storage.push(Item::new(value));
        self
    }

    pub fn build(mut self, value: S::Value, f: impl FnOnce(NodeBuilder<'_, Self>) -> NodeBuilder<'_, Self>) -> Self {
        let index = self.storage.len();
        self = self.add(value);
        f(NodeBuilder {
            b: &mut self,
            parent: Some(index),
            ld: 0,
        })
        .end();
        self
    }

    fn update(&mut self, index: usize, f: impl FnOnce(&mut Item<S::Value>)) -> &mut Self {
        f(self.storage.get_mut(index).unwrap());
        self
    }
}

impl<T, S> From<FlatTreeBuilder<S>> for FlatTree<T, S>
where
    S: Storage<Value = T>,
{
    fn from(builder: FlatTreeBuilder<S>) -> Self {
        builder.done()
    }
}

// ---

pub trait Build
where
    Self: Sized,
{
    type Storage: Storage;

    fn add(self, value: <<Self as Build>::Storage as Storage>::Value) -> Self;
    fn build(self, value: <<Self as Build>::Storage as Storage>::Value, f: impl FnOnce(Self) -> Self) -> Self;
}

// ---

pub struct NodeBuilder<'b, TB> {
    b: &'b mut TB,
    parent: Option<usize>,
    ld: usize,
}

impl<'b, S> NodeBuilder<'b, FlatTreeBuilder<S>>
where
    S: Storage,
{
    pub fn add(&mut self, value: S::Value) -> &mut Self {
        self.b.storage.push(Item {
            parent: self.parent,
            ..Item::new(value)
        });
        self.ld += 1;
        self
    }

    fn end(self) -> &'b mut FlatTreeBuilder<S> {
        if let Some(index) = self.parent {
            let lf = self.b.storage.len() - index;
            let ld = self.ld;
            self.b.update(index, |item| {
                item.lf = lf;
                item.ld = ld;
            })
        } else {
            self.b
        }
    }

    fn snapshot(self) -> (NodeBuilderSnapshot, &'b mut FlatTreeBuilder<S>) {
        (
            NodeBuilderSnapshot {
                parent: self.parent,
                ld: self.ld,
            },
            self.b,
        )
    }
}

impl<'b, S> Build for NodeBuilder<'b, FlatTreeBuilder<S>>
where
    S: Storage,
{
    type Storage = S;

    #[inline]
    fn add(mut self, value: S::Value) -> Self {
        NodeBuilder::add(&mut self, value);
        self
    }

    #[inline]
    fn build(mut self, value: S::Value, f: impl FnOnce(Self) -> Self) -> Self {
        let index = self.b.storage.len();
        NodeBuilder::add(&mut self, value);

        let (snapshot, b) = self.snapshot();

        let child = NodeBuilder {
            b,
            parent: Some(index),
            ld: 0,
        };

        let b = f(child).end();

        (snapshot, b).into()
    }
}

impl<'b, S> From<(NodeBuilderSnapshot, &'b mut FlatTreeBuilder<S>)> for NodeBuilder<'b, FlatTreeBuilder<S>>
where
    S: Storage,
{
    #[inline]
    fn from((state, b): (NodeBuilderSnapshot, &'b mut FlatTreeBuilder<S>)) -> Self {
        Self {
            b,
            parent: state.parent,
            ld: state.ld,
        }
    }
}

struct NodeBuilderSnapshot {
    parent: Option<usize>,
    ld: usize,
}

#[derive(Debug, Clone)]
pub struct Item<T> {
    value: T,
    parent: Option<usize>, // index of parent
    lf: usize,             // length (flat) - nubmer of direct and indirect children
    ld: usize,             // length (direct) - number of direct children
}

impl<T> Item<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            parent: None,
            lf: 0,
            ld: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test1() {
        let mut builder = FlatTree::<usize>::build();
        builder
            .root()
            .add(1)
            .add(2)
            .build(3, |b| b.add(4).add(5).build(6, |b| b.add(7).add(8)));
        let tree = builder.done();
        assert_eq!(tree.s.len(), 8);
    }

    #[test]
    fn test2() {
        let tree = FlatTree::<usize>::build()
            .add(1)
            .add(2)
            .build(3, |b| b.add(4).add(5).build(6, |b| b.add(7).add(8)))
            .add(9)
            .done();
        assert_eq!(tree.s.len(), 9);
    }
}
