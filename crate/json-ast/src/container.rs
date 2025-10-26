// workspace imports
use flat_tree::{FlatTree, tree};

// local imports
use crate::{
    error::Result,
    parse::parse_value,
    token::{self, Lexer},
};

// ---

#[derive(Default, Debug)]
pub struct Container<'s> {
    pub inner: ContainerInner<'s>,
}

impl<'s> Container<'s> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn parse(lexer: &mut Lexer<'s>) -> Result<Self> {
        let mut container = Self::new();
        container.extend(lexer)?;
        Ok(container)
    }

    #[inline]
    pub fn extend(&mut self, lexer: &mut Lexer<'s>) -> Result<()> {
        while let Some(_) = parse_value(lexer, self.inner.metaroot())? {}
        Ok(())
    }

    #[inline]
    pub fn nodes(&self) -> tree::Nodes<'_, Node<'s>> {
        self.inner.nodes()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }
}

// ---

pub trait Build<'s>: tree::Build<Value = Node<'s>> {}

impl<'s, T: tree::Build<Value = Node<'s>>> Build<'s> for T {}

pub trait BuildExt<'s>
where
    Self: Sized,
{
    type Child: BuildExt<'s>;

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
    type Child = T;

    #[inline]
    fn add_scalar(self, source: &'s str, kind: ScalarKind) -> Self {
        self.push(Node::new(NodeKind::Scalar(kind), source))
    }

    #[inline]
    fn add_object(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build(Node::new(NodeKind::Object, ""), f)
    }

    #[inline]
    fn add_array(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build(Node::new(NodeKind::Array, ""), f)
    }

    #[inline]
    fn add_field(self, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build(Node::new(NodeKind::Field, ""), f)
    }

    #[inline]
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
    #[inline]
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

impl<'s> From<token::String<'s>> for StringKind {
    #[inline]
    fn from(s: token::String<'s>) -> Self {
        match s {
            token::String::Plain(_) => Self::Plain,
            token::String::Escaped(_) => Self::Escaped,
        }
    }
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
}
