// std imports
use std::{marker::PhantomData, result::Result};

// third-party imports
use derive_where::derive_where;

// local imports
use crate::storage::Storage;

// ---

pub type DefaultStorage<V> = Vec<Item<V>>;

#[derive_where(Default; S: Default)]
#[derive_where(Debug)]
pub struct FlatTree<V, S = DefaultStorage<V>>
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
    pub fn new() -> Self {
        Self::with_storage(Default::default())
    }
}

impl<V, S> FlatTree<V, S>
where
    S: Storage<Value = V>,
{
    #[inline]
    pub fn with_storage(mut storage: S) -> Self {
        storage.clear();
        Self {
            storage,
            roots: 0,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn storage(&self) -> &S {
        &self.storage
    }

    #[inline]
    pub fn roots(&self) -> Roots<V, S> {
        Roots { tree: self }
    }

    #[inline]
    pub fn nodes(&self) -> Nodes<V, S> {
        Nodes {
            tree: self,
            start: 0,
            end: self.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.storage.clear();
        self.roots = 0;
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.storage.reserve(additional);
    }

    #[inline]
    pub fn metaroot(&mut self) -> NodeBuilder<V, S> {
        NodeBuilder {
            tree: self,
            index: None,
            children: 0,
        }
    }

    #[inline]
    pub fn with_node(mut self, value: S::Value) -> Self {
        (&mut self).push(value);
        self
    }

    #[inline]
    pub fn with_composite_node(
        mut self,
        value: S::Value,
        f: impl FnOnce(NodeBuilder<V, S>) -> NodeBuilder<V, S>,
    ) -> Self {
        (&mut self).build(value, f);
        self
    }

    #[inline]
    pub fn push(&mut self, value: S::Value) -> &mut Self {
        Build::push(self, value)
    }

    #[inline]
    pub fn build(&mut self, value: S::Value, f: impl FnOnce(NodeBuilder<V, S>) -> NodeBuilder<V, S>) -> &mut Self {
        Build::build(self, value, f)
    }

    #[inline]
    pub fn build_e<E>(
        &mut self,
        value: S::Value,
        f: impl FnOnce(NodeBuilder<V, S>) -> Result<NodeBuilder<V, S>, E>,
    ) -> Result<&mut Self, E> {
        Build::build_e(self, value, f)
    }

    #[inline]
    fn len(&self) -> usize {
        self.storage.len()
    }

    #[inline]
    fn node(&self, index: usize) -> Node<V, S> {
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

    #[inline]
    fn update(&mut self, index: usize, f: impl FnOnce(&mut Item<S::Value>)) -> &mut Self {
        f(self.storage.get_mut(index).unwrap());
        self
    }
}

impl<'t, V, S> Build for &'t mut FlatTree<V, S>
where
    S: Storage<Value = V>,
{
    type Value = V;
    type Storage = S;
    type Child = NodeBuilder<'t, V, S>;

    #[inline]
    fn push(self, value: S::Value) -> Self {
        self.storage.push(Item::new(value));
        self.roots += 1;
        self
    }

    #[inline]
    fn build_e<E>(mut self, value: S::Value, f: impl FnOnce(Self::Child) -> Result<Self::Child, E>) -> Result<Self, E> {
        let index = self.storage.len();
        self = self.push(value);

        let child = NodeBuilder {
            tree: self,
            index: Some(index),
            children: 0,
        };

        Ok(f(child)?.end())
    }
}

// ---

pub trait Build: Sized {
    type Value;
    type Storage: Storage<Value = Self::Value>;
    type Child: Build<Value = Self::Value, Storage = Self::Storage>;

    fn push(self, value: Self::Value) -> Self;
    fn build_e<E>(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Result<Self::Child, E>) -> Result<Self, E>;
    fn build(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Self::Child) -> Self {
        self.build_e(value, |child| Ok::<_, ()>(f(child))).unwrap()
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

#[derive_where(Clone, Copy)]
pub struct Nodes<'t, V, S = DefaultStorage<V>>
where
    S: Storage<Value = V>,
{
    tree: &'t FlatTree<V, S>,
    start: usize,
    end: usize,
}

impl<'t, V, S> Nodes<'t, V, S>
where
    S: Storage<Value = V>,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    #[inline]
    pub fn iter(&self) -> NodesIterator<'t, V, S> {
        self.into_iter()
    }
}

impl<'t, V, S> IntoIterator for Nodes<'t, V, S>
where
    S: Storage<Value = V>,
{
    type Item = Node<'t, V, S>;
    type IntoIter = NodesIterator<'t, V, S>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        NodesIterator {
            tree: self.tree,
            next: self.start,
            end: self.end,
        }
    }
}

pub struct NodesIterator<'t, V, S>
where
    S: Storage<Value = V>,
{
    tree: &'t FlatTree<V, S>,
    next: usize,
    end: usize,
}

impl<'t, V, S> Iterator for NodesIterator<'t, V, S>
where
    S: Storage<Value = V>,
{
    type Item = Node<'t, V, S>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }

        let node = self.tree.node(self.next);
        self.next += 1;

        Some(node)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.end - self.next;
        (n, Some(n))
    }

    #[inline]
    fn count(self) -> usize {
        self.end - self.next
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
    pub fn value(&self) -> &'t V {
        &self.item.value
    }

    #[inline]
    pub fn parent(&self) -> Option<Self> {
        self.item.parent.map(|index| self.tree.node(index))
    }

    #[inline]
    pub fn ancestors(&self) -> impl Iterator<Item = Self> + 't {
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
        if index < self.tree.len() {
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
    pub fn descendants(&self) -> Nodes<'t, V, S> {
        Nodes {
            tree: self.tree,
            start: self.index + 1,
            end: self.index + self.item.len,
        }
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

pub struct NodeBuilder<'t, V, S = DefaultStorage<V>>
where
    S: Storage<Value = V>,
{
    tree: &'t mut FlatTree<V, S>,
    index: Option<usize>,
    children: usize,
}

impl<'t, V, S> NodeBuilder<'t, V, S>
where
    S: Storage<Value = V>,
{
    #[inline]
    pub fn push(mut self, value: S::Value) -> Self {
        self.tree.storage.push(Item {
            parent: self.index,
            ..Item::new(value)
        });
        self.children += 1;
        if self.index.is_none() {
            self.tree.roots += 1;
        }
        self
    }

    #[inline]
    pub fn build(self, value: S::Value, f: impl FnOnce(Self) -> Self) -> Self {
        Build::build(self, value, f)
    }

    #[inline]
    pub fn build_e<E>(mut self, value: S::Value, f: impl FnOnce(Self) -> Result<Self, E>) -> Result<Self, E> {
        let index = self.tree.storage.len();
        self = self.push(value);

        let (snapshot, tree) = self.snapshot();

        let child = NodeBuilder {
            tree,
            index: Some(index),
            children: 0,
        };

        let tree = f(child)?.end();

        Ok((snapshot, tree).into())
    }

    #[inline]
    fn end(mut self) -> &'t mut FlatTree<S::Value, S> {
        self.close();
        self.tree
    }

    #[inline]
    fn close(&mut self) {
        if let Some(index) = self.index {
            let len = self.tree.storage.len() - index;
            let children = self.children;
            self.tree.update(index, |item| {
                item.len = len;
                item.children = children;
            });
        } else {
            self.tree.roots = self.children;
        }
    }

    #[inline]
    fn snapshot(self) -> (NodeBuilderSnapshot, &'t mut FlatTree<S::Value, S>)
    where
        Self: 't,
    {
        (
            NodeBuilderSnapshot {
                parent: self.index,
                children: self.children,
            },
            self.tree,
        )
    }
}

impl<'t, V, S> Build for NodeBuilder<'t, V, S>
where
    S: Storage<Value = V>,
{
    type Value = V;
    type Storage = S;
    type Child = Self;

    #[inline]
    fn push(self, value: S::Value) -> Self {
        self.push(value)
    }

    #[inline]
    fn build_e<E>(self, value: S::Value, f: impl FnOnce(Self) -> Result<Self, E>) -> Result<Self, E> {
        self.build_e(value, f)
    }
}

impl<'t, V, S> From<(NodeBuilderSnapshot, &'t mut FlatTree<V, S>)> for NodeBuilder<'t, V, S>
where
    S: Storage<Value = V>,
{
    #[inline]
    fn from((state, tree): (NodeBuilderSnapshot, &'t mut FlatTree<S::Value, S>)) -> Self {
        Self {
            tree,
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

    #[inline]
    pub fn parent(&self) -> Option<usize> {
        self.parent
    }

    #[inline]
    pub fn value(&self) -> &V {
        &self.value
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
        let tree = FlatTree::<i32>::new()
            .with_node(1)
            .with_node(2)
            .with_composite_node(3, |b| b.push(4).push(5).build(6, |b| b.push(7).push(8)))
            .with_node(9);
        assert_eq!(tree.storage.len(), 9);
        assert_eq!(tree.roots, 4);

        let node = tree.node(0);
        assert_eq!(*node.value(), 1);
        let descendants = collect(node.descendants());
        assert_eq!(descendants, []);

        let node = tree.node(6);
        assert_eq!(*node.value(), 7);
        let parents = collect(node.ancestors());
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

        let node = tree.node(2);
        let descendants = node.descendants();
        assert_eq!(descendants.len(), 5);
        assert_eq!(collect(descendants), [4, 5, 6, 7, 8]);

        assert_eq!(tree.roots().len(), 4);
        let roots = collect(tree.roots());
        assert_eq!(roots, [1, 2, 3, 9]);
        assert_eq!(tree.roots().iter().count(), 4);

        assert_eq!(tree.nodes().len(), 9);
        assert_eq!(tree.nodes().iter().count(), 9);
        let nodes = collect(tree.nodes());
        assert_eq!(nodes, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_tree_ref() {
        let mut tree = FlatTree::<usize>::new();
        let tree = &mut tree;
        tree.push(1)
            .push(2)
            .build(3, |b| b.push(4).push(5).build(6, |b| b.push(7).push(8)));
        assert_eq!(tree.storage.len(), 8);
        assert_eq!(tree.roots, 3);

        let roots = collect(tree.roots());
        assert_eq!(roots, [1, 2, 3]);
    }
}
