// std imports
use std::marker::PhantomData;

// local imports
use crate::storage::Storage;

// ---

pub struct FlatTree<T, S = Vec<Item<T>>>
where
    S: Storage<Value = T>,
{
    storage: S,
    ld: usize,
    _marker: PhantomData<T>,
}

impl<T, S> FlatTree<T, S>
where
    S: Storage<Value = T> + Default,
{
    #[inline]
    pub fn build() -> FlatTreeBuilder<S> {
        FlatTreeBuilder::new(Default::default())
    }
}

impl<T, S> FlatTree<T, S>
where
    S: Storage<Value = T>,
{
    #[inline]
    pub fn build_with_storage(mut storage: S) -> FlatTreeBuilder<S> {
        storage.clear();
        FlatTreeBuilder::new(storage)
    }

    #[inline]
    pub fn flat_len(&self) -> usize {
        self.storage.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    #[inline]
    fn node(&self, index: usize) -> Node<'_, T, S> {
        Node {
            tree: self,
            index,
            item: self.item(index),
        }
    }

    #[inline]
    fn item(&self, index: usize) -> &Item<T> {
        self.storage.get(index).unwrap()
    }
}

// ---

pub struct Node<'t, T, S>
where
    S: Storage<Value = T>,
{
    tree: &'t FlatTree<T, S>,
    index: usize,
    item: &'t Item<T>,
}

impl<'t, T, S> Node<'t, T, S>
where
    S: Storage<Value = T>,
{
    #[inline]
    pub fn value(&self) -> &T {
        &self.item.value
    }

    #[inline]
    pub fn parent(&self) -> Option<Self> {
        self.item.parent.map(|index| self.tree.node(index))
    }

    #[inline]
    pub fn parents(&self) -> impl Iterator<Item = Self> + 't {
        let tree = self.tree;
        let mut item = self.item;
        std::iter::from_fn(move || {
            let node = tree.node(item.parent?);
            item = node.item;
            Some(node)
        })
    }

    #[inline]
    pub fn next(&self) -> Option<Self> {
        let index = self.index + self.item.lf;
        if index < self.tree.flat_len() {
            Some(self.tree.node(index))
        } else {
            None
        }
    }

    #[inline]
    pub fn children(&self) -> impl Iterator<Item = Self> + 't {
        let tree = self.tree;
        let start = self.index + 1;
        let end = self.index + self.item.lf;
        let mut index = start;
        std::iter::from_fn(move || {
            if index < end {
                let node = tree.node(index);
                index += node.item.lf;
                Some(node)
            } else {
                None
            }
        })
    }

    #[inline]
    pub fn descendants(&self) -> impl Iterator<Item = Self> + 't {
        let tree = self.tree;
        let start = self.index + 1;
        let end = self.index + self.item.lf;
        (start..end).map(move |index| tree.node(index))
    }
}

// ---

pub struct FlatTreeBuilder<S> {
    storage: S,
    ld: usize,
}

impl<S> FlatTreeBuilder<S>
where
    S: Storage,
{
    #[inline]
    pub fn new(storage: S) -> Self {
        Self { storage, ld: 0 }
    }

    #[inline]
    pub fn roots<'b>(&'b mut self) -> NodeBuilder<'b, S> {
        NodeBuilder {
            builder: self,
            index: None,
            ld: 0,
        }
    }

    #[inline]
    pub fn done(self) -> FlatTree<S::Value, S> {
        FlatTree {
            storage: self.storage,
            ld: self.ld,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn add(mut self, value: S::Value) -> Self {
        self.storage.push(Item::new(value));
        self.ld += 1;
        self
    }

    #[inline]
    pub fn build(mut self, value: S::Value, f: impl FnOnce(NodeBuilder<'_, S>) -> NodeBuilder<'_, S>) -> Self {
        let index = self.storage.len();
        self = self.add(value);

        f(NodeBuilder {
            builder: &mut self,
            index: Some(index),
            ld: 0,
        })
        .end();

        self
    }

    #[inline]
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

pub struct NodeBuilder<'b, S>
where
    S: Storage,
{
    builder: &'b mut FlatTreeBuilder<S>,
    index: Option<usize>,
    ld: usize,
}

impl<'b, S> NodeBuilder<'b, S>
where
    S: Storage,
{
    #[inline]
    pub fn add(mut self, value: S::Value) -> Self {
        self.builder.storage.push(Item {
            parent: self.index,
            ..Item::new(value)
        });
        self.ld += 1;
        if self.index.is_none() {
            self.builder.ld += 1;
        }
        self
    }

    #[inline]
    pub fn build(mut self, value: S::Value, f: impl FnOnce(Self) -> Self) -> Self {
        let index = self.builder.storage.len();
        self = self.add(value);

        let (snapshot, b) = self.snapshot();

        let child = NodeBuilder {
            builder: b,
            index: Some(index),
            ld: 0,
        };

        let b = f(child).end();

        (snapshot, b).into()
    }

    #[inline]
    fn end(mut self) -> &'b mut FlatTreeBuilder<S> {
        self.close();
        self.builder
    }

    #[inline]
    fn close(&mut self) {
        if let Some(index) = self.index {
            let lf = self.builder.storage.len() - index;
            let ld = self.ld;
            self.builder.update(index, |item| {
                item.lf = lf;
                item.ld = ld;
            });
        } else {
            self.builder.ld = self.ld;
        }
    }

    #[inline]
    fn snapshot(self) -> (NodeBuilderSnapshot, &'b mut FlatTreeBuilder<S>)
    where
        Self: 'b,
    {
        (
            NodeBuilderSnapshot {
                parent: self.index,
                ld: self.ld,
            },
            self.builder,
        )
    }
}

impl<'b, S> From<(NodeBuilderSnapshot, &'b mut FlatTreeBuilder<S>)> for NodeBuilder<'b, S>
where
    S: Storage,
{
    #[inline]
    fn from((state, builder): (NodeBuilderSnapshot, &'b mut FlatTreeBuilder<S>)) -> Self {
        Self {
            builder,
            index: state.parent,
            ld: state.ld,
        }
    }
}

// ---

struct NodeBuilderSnapshot {
    parent: Option<usize>,
    ld: usize,
}

// ---

#[derive(Debug, Clone)]
pub struct Item<T> {
    value: T,
    parent: Option<usize>, // index of parent
    lf: usize,             // length (flat) - nubmer of direct and indirect children
    ld: usize,             // length (direct) - number of direct children
}

impl<T> Item<T> {
    #[inline]
    fn new(value: T) -> Self {
        Self {
            value,
            parent: None,
            lf: 1,
            ld: 0,
        }
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    fn collect<'t, T, S, I>(nodes: I) -> Vec<T>
    where
        I: Iterator<Item = Node<'t, T, S>> + 't,
        S: Storage<Value = T> + 't,
        T: Copy + 'static,
    {
        nodes.map(|n| *n.value()).collect()
    }

    #[test]
    fn test_tree() {
        let tree = FlatTree::<i32>::build()
            .add(1)
            .add(2)
            .build(3, |b| b.add(4).add(5).build(6, |b| b.add(7).add(8)))
            .add(9)
            .done();
        assert_eq!(tree.storage.len(), 9);
        assert_eq!(tree.ld, 4);

        let node = tree.node(0);
        assert_eq!(*node.value(), 1);
        let descendants = collect(node.descendants());
        assert_eq!(descendants, []);

        let node = tree.node(6);
        assert_eq!(*node.value(), 7);
        let parents = collect(node.parents());
        assert_eq!(parents, [6, 3]);

        let node = tree.node(2);
        assert_eq!(*node.value(), 3);
        let children = collect(node.children());
        assert_eq!(children, [4, 5, 6]);

        println!("tree: {:#?}", tree.storage);

        let node = tree.node(2);
        assert_eq!(*node.value(), 3);
        let next = node.next().unwrap();
        assert_eq!(*next.value(), 9);
        let next = next.next();
        assert!(next.is_none());
    }

    #[test]
    fn test_tree_roots() {
        let mut builder = FlatTree::<usize>::build();
        builder
            .roots()
            .add(1)
            .add(2)
            .build(3, |b| b.add(4).add(5).build(6, |b| b.add(7).add(8)));
        let tree = builder.done();
        assert_eq!(tree.storage.len(), 8);
        assert_eq!(tree.ld, 3);
    }
}
