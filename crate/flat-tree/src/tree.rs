// std imports
use std::{fmt::Debug, marker::PhantomData};

// third-party imports
use derive_where::derive_where;

// local imports
pub use super::build::*;
use super::{DefaultStorage, Index, OptIndex, Storage};

// ---

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

impl<V: Debug> FlatTree<V> {
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            storage: DefaultStorage::with_capacity(capacity),
            roots: 0,
            _marker: PhantomData,
        }
    }
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
            attachment: NoAttachment,
            index: None.into(),
        }
    }

    #[inline]
    pub fn push(&mut self, value: S::Value) -> &mut Self {
        Push::push(self, value)
    }

    #[inline]
    pub fn build<'s, R, F>(&'s mut self, value: S::Value, f: F) -> BuildOutput<F, R, NodeBuilder<'s, V, S>>
    where
        F: FnOnce(NodeBuilder<'s, V, S>) -> R,
        R: BuildFnResult<F, R, NodeBuilder<'s, V, S>>,
    {
        Build::build(self.metaroot(), value, f)
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
    fn update(&mut self, index: usize, f: impl FnOnce(&mut Item<S::Value>)) {
        f(self.storage.get_mut(index).unwrap());
    }
}

impl<'t, V, S> Push for &'t mut FlatTree<V, S>
where
    S: Storage<Value = V>,
{
    type Value = V;
    type Checkpoint = Checkpoint;

    #[inline]
    fn push(self, value: V) -> Self {
        if self.len() == usize::MAX - 1 {
            panic!("tree is full");
        }
        self.storage.push(Item::new(value));
        self.roots += 1;
        self
    }

    #[inline]
    fn checkpoint(&self) -> Self::Checkpoint {
        Checkpoint::new(self.len(), self.roots)
    }

    #[inline]
    fn rollback(&mut self, checkpoint: &Self::Checkpoint) {
        self.storage.truncate(checkpoint.len);
        self.roots = checkpoint.roots;
    }

    #[inline]
    fn next_index(&self) -> Index {
        Index(self.len())
    }

    #[inline]
    fn first_node_index(&self, checkpoint: &Self::Checkpoint) -> OptIndex {
        checkpoint.first_node_index(self)
    }
}

impl<'t, V, S> Reserve for &'t mut FlatTree<V, S>
where
    S: Storage<Value = V>,
{
    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.storage.reserve(additional);
    }
}

// ---

#[derive_where(Clone, Copy)]
pub struct Roots<'t, V, S = DefaultStorage<V>>
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
    pub fn iter(&self) -> SiblingsIter<'t, V, S> {
        self.into_iter()
    }
}

impl<'t, V, S> IntoIterator for Roots<'t, V, S>
where
    S: Storage<Value = V>,
{
    type Item = Node<'t, V, S>;
    type IntoIter = SiblingsIter<'t, V, S>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        SiblingsIter {
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
    pub fn get(&self, index: Index) -> Option<Node<'t, V, S>> {
        if (self.start..self.end).contains(&index.0) {
            Some(self.tree.node(index.0))
        } else {
            None
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    #[inline]
    pub fn iter(&self) -> NodesIter<'t, V, S> {
        self.into_iter()
    }
}

impl<'t, V, S> IntoIterator for Nodes<'t, V, S>
where
    S: Storage<Value = V>,
{
    type Item = Node<'t, V, S>;
    type IntoIter = NodesIter<'t, V, S>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        NodesIter {
            tree: self.tree,
            next: self.start,
            end: self.end,
        }
    }
}

pub struct NodesIter<'t, V, S>
where
    S: Storage<Value = V>,
{
    tree: &'t FlatTree<V, S>,
    next: usize,
    end: usize,
}

impl<'t, V, S> Iterator for NodesIter<'t, V, S>
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

pub struct SiblingsIter<'t, V, S = DefaultStorage<V>>
where
    S: Storage<Value = V>,
{
    tree: &'t FlatTree<V, S>,
    next: usize,
    n: usize,
}

impl<'t, V, S> Iterator for SiblingsIter<'t, V, S>
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
pub struct Node<'t, V, S = DefaultStorage<V>>
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
    pub fn index(&self) -> Index {
        Index(self.index)
    }

    #[inline]
    pub fn parent(&self) -> Option<Self> {
        self.item.parent.unfold().map(|index| self.tree.node(index.0))
    }

    #[inline]
    pub fn ancestors(&self) -> impl Iterator<Item = Self> + 't {
        let tree = self.tree;
        let mut item = self.item;
        std::iter::from_fn(move || {
            let node = tree.node(item.parent.unfold()?.0);
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

impl<'s, V, S> std::fmt::Debug for Node<'s, V, S>
where
    S: Storage<Value = V>,
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("value", self.value())
            .field("parent", &self.item.parent)
            .field("len", &self.item.len)
            .field("children", &self.item.children)
            .finish()
    }
}

// ---

#[derive_where(Clone, Copy)]
pub struct Children<'t, V, S = DefaultStorage<V>>
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
    pub fn iter(&self) -> SiblingsIter<'t, V, S> {
        self.into_iter()
    }
}

impl<'t, V, S> IntoIterator for Children<'t, V, S>
where
    S: Storage<Value = V>,
{
    type Item = Node<'t, V, S>;
    type IntoIter = SiblingsIter<'t, V, S>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        SiblingsIter {
            tree: self.tree,
            next: self.index + 1,
            n: self.n,
        }
    }
}

// ---

pub struct NodeBuilder<'t, V, S = DefaultStorage<V>, A = NoAttachment>
where
    S: Storage<Value = V>,
{
    tree: &'t mut FlatTree<V, S>,
    index: OptIndex,
    attachment: A,
}

impl<'t, V, S, A> NodeBuilder<'t, V, S, A>
where
    S: Storage<Value = V>,
    A: BuildAttachment,
{
    #[inline]
    pub fn push(self, value: S::Value) -> Self {
        self.tree.storage.push(Item {
            parent: self.index,
            ..Item::new(value)
        });
        match self.index.unfold() {
            Some(index) => self.tree.update(index.0, |item| item.children += 1),
            None => self.tree.roots += 1,
        }
        self
    }

    #[inline]
    pub fn build<R, F>(self, value: S::Value, f: F) -> BuildOutput<F, R, Self>
    where
        F: FnOnce(Self) -> R,
        R: BuildFnResult<F, R, Self>,
    {
        Build::build(self, value, f)
    }

    #[inline]
    fn end(mut self) -> (&'t mut FlatTree<S::Value, S>, A) {
        self.close();
        (self.tree, self.attachment)
    }

    #[inline]
    fn close(&mut self) {
        if let Some(index) = self.index.unfold() {
            let len = self.tree.storage.len() - index.0;
            self.tree.update(index.0, |item| {
                item.len = len;
            });
        }
    }

    #[inline]
    fn snapshot(self) -> (NodeBuilderSnapshot, A, &'t mut FlatTree<S::Value, S>) {
        (NodeBuilderSnapshot { parent: self.index }, self.attachment, self.tree)
    }
}

impl<'t, V, S, A> Push for NodeBuilder<'t, V, S, A>
where
    S: Storage<Value = V>,
    A: BuildAttachment,
{
    type Value = V;
    type Checkpoint = Checkpoint;

    #[inline]
    fn push(self, value: S::Value) -> Self {
        self.push(value)
    }

    #[inline]
    fn checkpoint(&self) -> Self::Checkpoint {
        Checkpoint::new(self.tree.len(), self.tree.roots)
    }

    #[inline]
    fn rollback(&mut self, checkpoint: &Self::Checkpoint) {
        self.tree.rollback(checkpoint)
    }

    #[inline]
    fn next_index(&self) -> Index {
        self.tree.next_index()
    }

    #[inline]
    fn first_node_index(&self, checkpoint: &Self::Checkpoint) -> OptIndex {
        checkpoint.first_node_index(self.tree)
    }
}

impl<'t, V, S, A> Reserve for NodeBuilder<'t, V, S, A>
where
    S: Storage<Value = V>,
{
    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.tree.reserve(additional);
    }
}

impl<'t, V, S, A> Build for NodeBuilder<'t, V, S, A>
where
    S: Storage<Value = V>,
    A: BuildAttachment,
{
    type Attachment = A;
    type WithAttachment<AV> = NodeBuilder<'t, V, S, A::Child<AV>>;
    type WithoutAttachment = NodeBuilder<'t, V, S, A::Parent>;

    #[inline]
    fn build<R, F>(mut self, value: V, f: F) -> BuildOutput<F, R, Self>
    where
        F: FnOnce(Self) -> R,
        R: BuildFnResult<F, R, Self>,
    {
        let index = Index(self.tree.storage.len());
        self = self.push(value);

        let (snapshot, attachment, tree) = self.snapshot();

        let child = NodeBuilder {
            tree,
            attachment,
            index: Some(index).into(),
        };

        f(child).transform(|child| {
            let (tree, attachment) = child.end();
            NodeBuilder::from((snapshot, attachment, tree))
        })
    }

    #[inline]
    fn attach<AV>(self, attachment: AV) -> Self::WithAttachment<AV> {
        NodeBuilder {
            tree: self.tree,
            index: self.index,
            attachment: self.attachment.join(attachment),
        }
    }

    #[inline]
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<A>) {
        let (attachment, value) = self.attachment.split();
        let builder = NodeBuilder {
            tree: self.tree,
            index: self.index,
            attachment,
        };
        (builder, value)
    }
}

impl<'t, V, S, A> From<(NodeBuilderSnapshot, A, &'t mut FlatTree<V, S>)> for NodeBuilder<'t, V, S, A>
where
    S: Storage<Value = V>,
    A: BuildAttachment,
{
    #[inline]
    fn from((state, attachment, tree): (NodeBuilderSnapshot, A, &'t mut FlatTree<S::Value, S>)) -> Self {
        Self {
            tree,
            index: state.parent,
            attachment,
        }
    }
}

// ---

#[derive(Debug, Clone)]
pub struct Checkpoint {
    len: usize,
    roots: usize,
}

impl Checkpoint {
    #[inline]
    fn new(len: usize, roots: usize) -> Self {
        Self { len, roots }
    }

    #[inline]
    fn first_node_index<V, S: Storage<Value = V>>(&self, tree: &FlatTree<V, S>) -> OptIndex {
        if self.len < tree.storage.len() {
            OptIndex::new(Some(Index(self.len)))
        } else {
            None.into()
        }
    }
}

// ---

struct NodeBuilderSnapshot {
    parent: OptIndex,
}

// ---

#[derive(Debug, Clone)]
pub struct Item<V> {
    value: V,
    parent: OptIndex, // index of parent
    len: usize,       // length (flat) - number of items including this item, its direct and indirect children
    children: usize,  // number of direct children
}

impl<V> Item<V> {
    #[inline]
    fn new(value: V) -> Self {
        Self {
            value,
            parent: None.into(),
            len: 1,
            children: 0,
        }
    }

    #[inline]
    pub fn parent(&self) -> OptIndex {
        self.parent
    }

    #[inline]
    pub fn value(&self) -> &V {
        &self.value
    }
}

// ---

pub struct BuilderAttachment<P, V> {
    parent: P,
    value: V,
}

impl<P, V> BuildAttachment for BuilderAttachment<P, V>
where
    P: BuildAttachment,
{
    type Parent = P;
    type Child<V2> = BuilderAttachment<Self, V2>;
    type Value = V;

    fn join<V2>(self, value: V2) -> Self::Child<V2> {
        BuilderAttachment { parent: self, value }
    }

    fn split(self) -> (Self::Parent, Self::Value) {
        (self.parent, self.value)
    }
}

// ---

pub struct NoAttachment;

impl BuildAttachment for NoAttachment {
    type Parent = NoAttachment;
    type Child<V> = BuilderAttachment<Self, V>;
    type Value = ();

    fn join<V>(self, value: V) -> Self::Child<V> {
        BuilderAttachment { parent: self, value }
    }

    fn split(self) -> (Self::Parent, Self::Value) {
        (self, ())
    }
}

// ---

#[cfg(test)]
mod tests {
    use core::panic;

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
    fn test_tree_attach() {
        fn check(_: &NodeBuilder<i32>) {}

        let mut tree = FlatTree::<i32>::new();
        let builder = tree.metaroot();
        check(&builder);
        let (builder, attachment) = builder.attach("aaa").push(1).push(2).push(3).detach();
        check(&builder);
        assert_eq!(attachment, "aaa");
    }

    #[test]
    fn test_tree_attach_nested() {
        fn check(_: &NodeBuilder<i32>) {}

        let mut tree = FlatTree::<i32>::new();
        let builder = tree.metaroot();
        check(&builder);
        let (builder, attachment) = builder
            .push(1)
            .attach("aaa")
            .build(2, |b| b.detach().0.push(3).attach("bbb"))
            .detach();
        check(&builder);
        assert_eq!(attachment, "bbb");
    }

    #[test]
    fn test_tree() {
        let mut tree = FlatTree::<i32>::new();
        tree.push(1).push(2).build(3, |b| b.push(4).push(5).push(6)).push(9);

        let x = tree.metaroot();
        x.build(10, |b| b.push(11).push(12).build(13, |b| b.push(14).push(15)));

        let x = tree.metaroot();
        let r = x.build(11, |b| {
            b.push(12).push(13).build(14, |b| {
                b.push(15).push(16);
                Err("some error")
            })
        });
        match r {
            Ok(_) => panic!("expected error"),
            Err(e) => assert_eq!(e, "some error"),
        }

        let mut tree = FlatTree::<i32>::new();
        tree.push(1)
            .push(2)
            .build(3, |b| b.push(4).push(5).build(6, |b| b.push(7).push(8)))
            .push(9);
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

    #[test]
    fn test_result_tuple() {
        let mut tree = FlatTree::<usize>::new();
        let b = tree.metaroot();
        let (_, result) = b.push(1).build(2, |b| (b.push(3), true));
        assert_eq!(tree.storage.len(), 3);
        assert_eq!(tree.roots, 2);
        assert_eq!(result, true);

        let roots = collect(tree.roots());
        assert_eq!(roots, [1, 2]);
    }
}
