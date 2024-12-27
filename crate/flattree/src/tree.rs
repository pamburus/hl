// std imports
use std::marker::PhantomData;

// local imports
use crate::storage::Storage;

pub struct FlatTree<T, S = Vec<Item<T>>>
where
    S: Storage<Value = T>,
{
    storage: S,
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
            builder: self,
            parent: None,
            ld: 0,
        }
    }

    pub fn done(self) -> FlatTree<S::Value, S> {
        FlatTree {
            storage: self.storage,
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
            builder: &mut self,
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

pub struct NodeBuilder<'b, B> {
    builder: &'b mut B,
    parent: Option<usize>,
    ld: usize,
}

impl<'b, S> NodeBuilder<'b, FlatTreeBuilder<S>>
where
    S: Storage,
{
    pub fn add(mut self, value: S::Value) -> Self {
        self.builder.storage.push(Item {
            parent: self.parent,
            ..Item::new(value)
        });
        self.ld += 1;
        self
    }

    #[inline]
    pub fn build(mut self, value: S::Value, f: impl FnOnce(Self) -> Self) -> Self {
        let index = self.builder.storage.len();
        self = self.add(value);

        let (snapshot, b) = self.snapshot();

        let child = NodeBuilder {
            builder: b,
            parent: Some(index),
            ld: 0,
        };

        let b = f(child).end();

        (snapshot, b).into()
    }

    fn end(self) -> &'b mut FlatTreeBuilder<S> {
        if let Some(index) = self.parent {
            let lf = self.builder.storage.len() - index;
            let ld = self.ld;
            self.builder.update(index, |item| {
                item.lf = lf;
                item.ld = ld;
            })
        } else {
            self.builder
        }
    }

    fn snapshot(self) -> (NodeBuilderSnapshot, &'b mut FlatTreeBuilder<S>) {
        (
            NodeBuilderSnapshot {
                parent: self.parent,
                ld: self.ld,
            },
            self.builder,
        )
    }
}

impl<'b, S> From<(NodeBuilderSnapshot, &'b mut FlatTreeBuilder<S>)> for NodeBuilder<'b, FlatTreeBuilder<S>>
where
    S: Storage,
{
    #[inline]
    fn from((state, builder): (NodeBuilderSnapshot, &'b mut FlatTreeBuilder<S>)) -> Self {
        Self {
            builder,
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
        assert_eq!(tree.storage.len(), 8);
    }

    #[test]
    fn test2() {
        let tree = FlatTree::<usize>::build()
            .add(1)
            .add(2)
            .build(3, |b| b.add(4).add(5).build(6, |b| b.add(7).add(8)))
            .add(9)
            .done();
        assert_eq!(tree.storage.len(), 9);
    }
}
