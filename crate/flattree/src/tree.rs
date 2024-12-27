use crate::storage::{Storage, StorageType};

pub struct FlatTree<S>
where
    S: StorageType,
{
    s: S::Storage,
}

impl<S> FlatTree<S>
where
    S: StorageType,
    S::Storage: Default,
{
    pub fn build() -> FlatTreeBuilder<S> {
        FlatTreeBuilder::new(Default::default())
    }
}

impl<S> FlatTree<S>
where
    S: StorageType,
{
    pub fn build_with(storage: S::Storage) -> FlatTreeBuilder<S> {
        FlatTreeBuilder::new(storage)
    }
}

pub struct FlatTreeBuilder<S>
where
    S: StorageType,
{
    s: S::Storage,
}

impl<S> FlatTreeBuilder<S>
where
    S: StorageType,
{
    pub fn new(storage: S::Storage) -> Self {
        Self { s: storage }
    }

    pub fn root<'b>(&'b mut self) -> NodeBuilder<'b, Self> {
        NodeBuilder {
            b: self,
            parent: None,
            ld: 0,
        }
    }

    pub fn done(self) -> FlatTree<S> {
        FlatTree { s: self.s }
    }

    pub fn add(&mut self, value: S::Value) -> &mut Self {
        self.s.push(Item::new(value));
        self
    }

    fn update(&mut self, index: usize, f: impl FnOnce(&mut Item<S::Value>)) -> &mut Self {
        f(self.s.get_mut(index).unwrap());
        self
    }
}

impl<S> From<FlatTreeBuilder<S>> for FlatTree<S>
where
    S: StorageType,
{
    fn from(builder: FlatTreeBuilder<S>) -> Self {
        builder.done()
    }
}

// ---

pub trait Build<S: StorageType>
where
    Self: Sized,
{
    fn add(self, value: S::Value) -> Self;
    fn build(self, value: S::Value, f: impl FnOnce(Self) -> Self) -> Self;
}

// ---

pub struct NodeBuilder<'b, TB> {
    b: &'b mut TB,
    parent: Option<usize>,
    ld: usize,
}

impl<'b, S> NodeBuilder<'b, FlatTreeBuilder<S>>
where
    S: StorageType,
{
    pub fn add(&mut self, value: S::Value) -> &mut Self {
        self.b.s.push(Item {
            parent: self.parent,
            ..Item::new(value)
        });
        self.ld += 1;
        self
    }

    fn end(self) -> &'b mut FlatTreeBuilder<S> {
        if let Some(index) = self.parent {
            let lf = self.b.s.len() - index;
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

impl<'b, S> Build<S> for NodeBuilder<'b, FlatTreeBuilder<S>>
where
    S: StorageType,
{
    #[inline]
    fn add(mut self, value: S::Value) -> Self {
        NodeBuilder::add(&mut self, value);
        self
    }

    #[inline]
    fn build(mut self, value: S::Value, f: impl FnOnce(Self) -> Self) -> Self {
        let index = self.b.s.len();
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
    S: StorageType,
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
    fn test() {
        let mut builder = FlatTree::<Vec<_>>::build();
        builder
            .root()
            .add(1)
            .add(2)
            .build(3, |b| b.add(4).add(5).build(6, |b| b.add(7).add(8)));
        let tree = builder.done();
        assert_eq!(tree.s.len(), 8);
    }
}
