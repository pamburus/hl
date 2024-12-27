use crate::{
    domain::{DefaultDomain, Domain},
    storage::Storage,
};

pub struct FlatTree<T, D = DefaultDomain<T>>
where
    D: Domain<Value = T>,
{
    s: D::Storage,
}

impl<T, D> FlatTree<T, D>
where
    D: Domain<Value = T>,
    D::Storage: Default,
{
    pub fn build() -> FlatTreeBuilder<D> {
        FlatTreeBuilder::new(Default::default())
    }
}

impl<T, D> FlatTree<T, D>
where
    D: Domain<Value = T>,
{
    pub fn build_with(storage: D::Storage) -> FlatTreeBuilder<D> {
        FlatTreeBuilder::new(storage)
    }
}

pub struct FlatTreeBuilder<D>
where
    D: Domain,
{
    s: D::Storage,
}

impl<D> FlatTreeBuilder<D>
where
    D: Domain,
{
    pub fn new(storage: D::Storage) -> Self {
        Self { s: storage }
    }

    pub fn root<'b>(&'b mut self) -> NodeBuilder<'b, Self> {
        NodeBuilder {
            b: self,
            parent: None,
            ld: 0,
        }
    }

    pub fn done(self) -> FlatTree<D::Value, D> {
        FlatTree { s: self.s }
    }

    pub fn add(&mut self, value: D::Value) -> &mut Self {
        self.s.push(Item::new(value));
        self
    }

    pub fn build(&mut self, value: D::Value, f: impl FnOnce(&mut Self) -> &mut Self) -> &mut Self {
        let index = self.s.len();
        self.add(value);
        f(self).update(index, |item| {
            item.parent = None;
        });
        self
    }

    fn update(&mut self, index: usize, f: impl FnOnce(&mut Item<D::Value>)) -> &mut Self {
        f(self.s.get_mut(index).unwrap());
        self
    }
}

impl<T, D> From<FlatTreeBuilder<D>> for FlatTree<T, D>
where
    D: Domain<Value = T>,
{
    fn from(builder: FlatTreeBuilder<D>) -> Self {
        builder.done()
    }
}

// ---

pub trait Build<D: Domain>
where
    Self: Sized,
{
    fn add(self, value: D::Value) -> Self;
    fn build(self, value: D::Value, f: impl FnOnce(Self) -> Self) -> Self;
}

// ---

pub struct NodeBuilder<'b, TB> {
    b: &'b mut TB,
    parent: Option<usize>,
    ld: usize,
}

impl<'b, D> NodeBuilder<'b, FlatTreeBuilder<D>>
where
    D: Domain,
{
    pub fn add(&mut self, value: D::Value) -> &mut Self {
        self.b.s.push(Item {
            parent: self.parent,
            ..Item::new(value)
        });
        self.ld += 1;
        self
    }

    fn end(self) -> &'b mut FlatTreeBuilder<D> {
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

    fn snapshot(self) -> (NodeBuilderSnapshot, &'b mut FlatTreeBuilder<D>) {
        (
            NodeBuilderSnapshot {
                parent: self.parent,
                ld: self.ld,
            },
            self.b,
        )
    }
}

impl<'b, D> Build<D> for NodeBuilder<'b, FlatTreeBuilder<D>>
where
    D: Domain,
{
    #[inline]
    fn add(mut self, value: D::Value) -> Self {
        NodeBuilder::add(&mut self, value);
        self
    }

    #[inline]
    fn build(mut self, value: D::Value, f: impl FnOnce(Self) -> Self) -> Self {
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

impl<'b, D> From<(NodeBuilderSnapshot, &'b mut FlatTreeBuilder<D>)> for NodeBuilder<'b, FlatTreeBuilder<D>>
where
    D: Domain,
{
    #[inline]
    fn from((state, b): (NodeBuilderSnapshot, &'b mut FlatTreeBuilder<D>)) -> Self {
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
}
