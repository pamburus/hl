// std imports
use std::marker::PhantomData;

// third-party imports
use derive_where::derive_where;

// local imports
use crate::storage::Storage;

// ---

pub struct FlatTree<V, S = Vec<Item<V>>>
where
    S: Storage<Value = V>,
{
    storage: S,
    roots: usize,
    _marker: PhantomData<V>,
}

impl<V, S> FlatTree<V, S>
where
    S: Storage<Value = V> + Default,
{
    #[inline]
    pub fn build() -> FlatTreeBuilder<S> {
        FlatTreeBuilder::new(Default::default())
    }
}

impl<V, S> FlatTree<V, S>
where
    S: Storage<Value = V>,
{
    #[inline]
    pub fn build_with_storage(mut storage: S) -> FlatTreeBuilder<S> {
        storage.clear();
        FlatTreeBuilder::new(storage)
    }

    #[inline]
    pub fn roots(&self) -> Roots<'_, V, S> {
        Roots { tree: self }
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
    fn node(&self, index: usize) -> Node<'_, V, S> {
        Node {
            tree: self,
            index,
            item: self.item(index),
        }
    }

    #[inline]
    fn item(&self, index: usize) -> &Item<V> {
        self.storage.get(index).unwrap()
    }
}

// ---

#[derive_where(Clone, Copy)]
pub struct Roots<'t, V, S>
where
    S: Storage<Value = V>,
{
    tree: &'t FlatTree<V, S>,
}

impl<'t, V, S> Roots<'t, V, S>
where
    S: Storage<Value = V>,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.tree.roots
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> SiblingsIterator<'t, V, S> {
        self.into_iter()
    }
}

impl<'t, V, S> IntoIterator for Roots<'t, V, S>
where
    S: Storage<Value = V>,
{
    type Item = Node<'t, V, S>;
    type IntoIter = SiblingsIterator<'t, V, S>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        SiblingsIterator {
            tree: self.tree,
            next: 0,
            n: self.tree.roots,
        }
    }
}

// ---

pub struct SiblingsIterator<'t, V, S>
where
    S: Storage<Value = V>,
{
    tree: &'t FlatTree<V, S>,
    next: usize,
    n: usize,
}

impl<'t, V, S> Iterator for SiblingsIterator<'t, V, S>
where
    S: Storage<Value = V>,
{
    type Item = Node<'t, V, S>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 {
            return None;
        }

        let node = self.tree.node(self.next);
        self.next += node.item.len;
        self.n -= 1;

        Some(node)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.n, Some(self.n))
    }

    #[inline]
    fn count(self) -> usize {
        self.n
    }
}

// ---

#[derive_where(Clone, Copy)]
pub struct Node<'t, V, S>
where
    S: Storage<Value = V>,
{
    tree: &'t FlatTree<V, S>,
    index: usize,
    item: &'t Item<V>,
}

impl<'t, V, S> Node<'t, V, S>
where
    S: Storage<Value = V>,
{
    #[inline]
    pub fn value(&self) -> &V {
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
        let index = self.index + self.item.len;
        if index < self.tree.flat_len() {
            Some(self.tree.node(index))
        } else {
            None
        }
    }

    #[inline]
    pub fn children(&self) -> Children<'t, V, S> {
        Children {
            tree: self.tree,
            index: self.index,
            n: self.item.children,
        }
    }

    #[inline]
    pub fn descendants(&self) -> impl Iterator<Item = Self> + 't {
        let tree = self.tree;
        let start = self.index + 1;
        let end = self.index + self.item.len;
        (start..end).map(move |index| tree.node(index))
    }
}

// ---

#[derive_where(Clone, Copy)]
pub struct Children<'t, V, S>
where
    S: Storage<Value = V>,
{
    tree: &'t FlatTree<V, S>,
    index: usize,
    n: usize,
}

impl<'t, V, S> Children<'t, V, S>
where
    S: Storage<Value = V>,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.n
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    #[inline]
    pub fn iter(&self) -> SiblingsIterator<'t, V, S> {
        self.into_iter()
    }
}

impl<'t, V, S> IntoIterator for Children<'t, V, S>
where
    S: Storage<Value = V>,
{
    type Item = Node<'t, V, S>;
    type IntoIter = SiblingsIterator<'t, V, S>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        SiblingsIterator {
            tree: self.tree,
            next: self.index + 1,
            n: self.n,
        }
    }
}

// ---

pub struct FlatTreeBuilder<S> {
    storage: S,
    roots: usize,
}

impl<S> FlatTreeBuilder<S>
where
    S: Storage,
{
    #[inline]
    pub fn new(storage: S) -> Self {
        Self { storage, roots: 0 }
    }

    #[inline]
    pub fn roots<'b>(&'b mut self) -> NodeBuilder<'b, S> {
        NodeBuilder {
            builder: self,
            index: None,
            children: 0,
        }
    }

    #[inline]
    pub fn done(self) -> FlatTree<S::Value, S> {
        FlatTree {
            storage: self.storage,
            roots: self.roots,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn add(mut self, value: S::Value) -> Self {
        self.storage.push(Item::new(value));
        self.roots += 1;
        self
    }

    #[inline]
    pub fn build(mut self, value: S::Value, f: impl FnOnce(NodeBuilder<'_, S>) -> NodeBuilder<'_, S>) -> Self {
        let index = self.storage.len();
        self = self.add(value);

        f(NodeBuilder {
            builder: &mut self,
            index: Some(index),
            children: 0,
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
    children: usize,
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
        self.children += 1;
        if self.index.is_none() {
            self.builder.roots += 1;
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
            children: 0,
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
            let len = self.builder.storage.len() - index;
            let children = self.children;
            self.builder.update(index, |item| {
                item.len = len;
                item.children = children;
            });
        } else {
            self.builder.roots = self.children;
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
                children: self.children,
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
            children: state.children,
        }
    }
}

// ---

struct NodeBuilderSnapshot {
    parent: Option<usize>,
    children: usize,
}

// ---

#[derive(Debug, Clone)]
pub struct Item<V> {
    value: V,
    parent: Option<usize>, // index of parent
    len: usize,            // length (flat) - number of items including this item, its direct and indirect children
    children: usize,       // number of direct children
}

impl<V> Item<V> {
    #[inline]
    fn new(value: V) -> Self {
        Self {
            value,
            parent: None,
            len: 1,
            children: 0,
        }
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    fn collect<'t, V, S, I>(nodes: I) -> Vec<V>
    where
        I: IntoIterator<Item = Node<'t, V, S>> + 't,
        S: Storage<Value = V> + 't,
        V: Copy + 'static,
    {
        nodes.into_iter().map(|n| *n.value()).collect()
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
        assert_eq!(tree.roots, 4);

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
        assert_eq!(node.children().len(), 3);
        let children = collect(node.children());
        assert_eq!(children, [4, 5, 6]);

        let node = tree.node(2);
        assert_eq!(*node.value(), 3);
        let next = node.next().unwrap();
        assert_eq!(*next.value(), 9);
        let next = next.next();
        assert!(next.is_none());

        assert_eq!(tree.roots().len(), 4);
        let roots = collect(tree.roots());
        assert_eq!(roots, [1, 2, 3, 9]);
        assert_eq!(tree.roots().iter().count(), 4);
    }

    #[test]
    fn test_tree_builder_roots() {
        let mut builder = FlatTree::<usize>::build();
        builder
            .roots()
            .add(1)
            .add(2)
            .build(3, |b| b.add(4).add(5).build(6, |b| b.add(7).add(8)));
        let tree = builder.done();
        assert_eq!(tree.storage.len(), 8);
        assert_eq!(tree.roots, 3);

        let roots = collect(tree.roots());
        assert_eq!(roots, [1, 2, 3]);
    }
}
