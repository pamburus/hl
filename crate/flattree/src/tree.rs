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
    #[inline(always)]
    pub fn build() -> FlatTreeBuilder<S> {
        FlatTreeBuilder::new(Default::default())
    }
}

impl<V, S> FlatTree<V, S>
where
    S: Storage<Value = V>,
{
    #[inline(always)]
    pub fn build_with_storage(mut storage: S) -> FlatTreeBuilder<S> {
        storage.clear();
        FlatTreeBuilder::new(storage)
    }

    #[inline(always)]
    pub fn storage(&self) -> &S {
        &self.storage
    }

    #[inline(always)]
    pub fn roots(&self) -> Roots<'_, V, S> {
        Roots { tree: self }
    }

    #[inline(always)]
    pub fn nodes(&self) -> Nodes<'_, V, S> {
        Nodes {
            tree: self,
            start: 0,
            end: self.len(),
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.storage.len()
    }

    #[inline(always)]
    fn node(&self, index: usize) -> Node<'_, V, S> {
        Node {
            tree: self,
            index,
            item: self.item(index),
        }
    }

    #[inline(always)]
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
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.tree.roots
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    #[inline(always)]
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

    #[inline(always)]
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
pub struct Nodes<'t, V, S>
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
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }

        let node = self.tree.node(self.next);
        self.next += 1;

        Some(node)
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.end - self.next;
        (n, Some(n))
    }

    #[inline(always)]
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

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 {
            return None;
        }

        let node = self.tree.node(self.next);
        self.next += node.item.len;
        self.n -= 1;

        Some(node)
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.n, Some(self.n))
    }

    #[inline(always)]
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
    #[inline(always)]
    pub fn value(&self) -> &'t V {
        &self.item.value
    }

    #[inline(always)]
    pub fn parent(&self) -> Option<Self> {
        self.item.parent.map(|index| self.tree.node(index))
    }

    #[inline(always)]
    pub fn ancestors(&self) -> impl Iterator<Item = Self> + 't {
        let tree = self.tree;
        let mut item = self.item;
        std::iter::from_fn(move || {
            let node = tree.node(item.parent?);
            item = node.item;
            Some(node)
        })
    }

    #[inline(always)]
    pub fn next(&self) -> Option<Self> {
        let index = self.index + self.item.len;
        if index < self.tree.len() {
            Some(self.tree.node(index))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn children(&self) -> Children<'t, V, S> {
        Children {
            tree: self.tree,
            index: self.index,
            n: self.item.children,
        }
    }

    #[inline(always)]
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
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.n
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    #[inline(always)]
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

    #[inline(always)]
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
    #[inline(always)]
    pub fn new(storage: S) -> Self {
        Self { storage, roots: 0 }
    }

    #[inline(always)]
    pub fn roots<'b>(&'b mut self) -> NodeBuilder<'b, S> {
        NodeBuilder {
            builder: self,
            index: None,
            children: 0,
        }
    }

    #[inline(always)]
    pub fn done(self) -> FlatTree<S::Value, S> {
        FlatTree {
            storage: self.storage,
            roots: self.roots,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub fn add(mut self, value: S::Value) -> Self {
        self.storage.push(Item::new(value));
        self.roots += 1;
        self
    }

    #[inline(always)]
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

    #[inline(always)]
    fn update(&mut self, index: usize, f: impl FnOnce(&mut Item<S::Value>)) -> &mut Self {
        f(self.storage.get_mut(index).unwrap());
        self
    }
}

impl<T, S> From<FlatTreeBuilder<S>> for FlatTree<T, S>
where
    S: Storage<Value = T>,
{
    #[inline(always)]
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
    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
    fn end(mut self) -> &'b mut FlatTreeBuilder<S> {
        self.close();
        self.builder
    }

    #[inline(always)]
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

    #[inline(always)]
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
    #[inline(always)]
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
    #[inline(always)]
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
