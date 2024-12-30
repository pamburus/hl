// workspace imports
use flat_tree::{tree, FlatTree};

// local imports
use crate::{error::Result, parse::parse_value, token::Lexer};

// ---

#[derive(Default, Debug)]
pub struct Container<'s> {
    pub inner: ContainerInner<'s>,
}

impl<'s> Container<'s> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(lexer: &mut Lexer<'s>) -> Result<Self> {
        let mut container = Self::new();
        container.extend(lexer)?;
        Ok(container)
    }

    pub fn extend(&mut self, lexer: &mut Lexer<'s>) -> Result<()> {
        while let Some(_) = parse_value(lexer, self.inner.metaroot())? {}
        Ok(())
    }

    pub fn nodes(&self) -> tree::Nodes<Node<'s>> {
        self.inner.nodes()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }
}

// ---

pub trait Build<'s>: tree::Build<Value = Node<'s>> {}

impl<'s, T: tree::Build<Value = Node<'s>>> Build<'s> for T {}

pub trait BuildExt<'s>: Build<'s> {
    fn add_scalar(self, source: &'s str, kind: ScalarKind) -> Self;
    fn add_object(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_array(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_field(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_key(self, source: &'s str, kind: StringKind) -> Self;
}

impl<'s, T> BuildExt<'s> for T
where
    T: Build<'s>,
{
    fn add_scalar(self, source: &'s str, kind: ScalarKind) -> Self {
        self.push(Node::new(NodeKind::Scalar(kind), source))
    }

    fn add_object(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Node::new(NodeKind::Object, ""), f)
    }

    fn add_array(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Node::new(NodeKind::Array, ""), f)
    }

    fn add_field(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Node::new(NodeKind::Field, ""), f)
    }

    fn add_key(self, source: &'s str, kind: StringKind) -> Self {
        self.push(Node::new(NodeKind::Key(kind), source))
    }
}

// ---

pub type ContainerInner<'s> = FlatTree<Node<'s>>;

// ---

#[derive(Debug)]
pub struct Node<'s> {
    kind: NodeKind,
    source: &'s str,
}

impl<'s> Node<'s> {
    pub fn new(kind: NodeKind, source: &'s str) -> Self {
        Self { kind, source }
    }
}

#[derive(Debug)]
pub enum NodeKind {
    Scalar(ScalarKind),
    Array,
    Object,
    Field,
    Key(StringKind),
}

#[derive(Debug)]
pub enum ScalarKind {
    Null,
    Bool(bool),
    Number,
    String(StringKind),
}

#[derive(Debug)]
pub enum StringKind {
    Plain,
    Escaped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container() {
        let mut lexer = Lexer::new(r#"{"key": "value"}"#);
        let container = Container::parse(&mut lexer).unwrap();
        assert_eq!(container.nodes().len(), 4);
    }

    #[bench]
    fn bench_container(b: &mut test::Bencher) {
        let mut lexer = Lexer::new(r#"{"key": "value"}"#);
        b.iter(|| {
            let container = Container::parse(&mut lexer).unwrap();
            assert_eq!(container.nodes().len(), 4);
        });
    }
}
