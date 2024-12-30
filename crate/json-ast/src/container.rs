// workspace imports
use flat_tree::{tree, FlatTree};

// local imports
use crate::{error::Result, parse::parse_value, token::Lexer};

// ---

#[derive(Default, Debug)]
pub struct Container<'s> {
    pub(crate) inner: ContainerInner<'s>,
}

impl<'s> Container<'s> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(lexer: &mut Lexer<'s>) -> Result<Self> {
        let mut container = Self::new();
        while let Some(_) = parse_value(lexer, container.inner.metaroot())? {}
        Ok(container)
    }
}

// ---

pub(crate) trait Build<'s>: tree::Build<Value = Node<'s>> {}

impl<'s, T: tree::Build<Value = Node<'s>>> Build<'s> for T {}

pub(crate) trait BuildExt<'s>: Build<'s> {
    fn add_scalar(self, source: &'s str, kind: ScalarKind) -> Self;
    fn add_object(self, source: &'s str, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_array(self, source: &'s str, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_field(self, source: &'s str, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self>;
    fn add_key(self, source: &'s str, kind: StringKind) -> Self;
}

impl<'s, T> BuildExt<'s> for T
where
    T: Build<'s>,
{
    fn add_scalar(self, source: &'s str, kind: ScalarKind) -> Self {
        self.push(Node::new(NodeKind::Scalar(kind), source))
    }

    fn add_object(self, source: &'s str, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Node::new(NodeKind::Object, source), f)
    }

    fn add_array(self, source: &'s str, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Node::new(NodeKind::Array, source), f)
    }

    fn add_field(self, source: &'s str, f: impl FnOnce(Self::Child) -> Result<Self::Child>) -> Result<Self> {
        self.build_e(Node::new(NodeKind::Field, source), f)
    }

    fn add_key(self, source: &'s str, kind: StringKind) -> Self {
        self.push(Node::new(NodeKind::Key(kind), source))
    }
}

// ---

pub(crate) type ContainerInner<'s> = FlatTree<Node<'s>>;

// ---

#[derive(Debug)]
pub(crate) struct Node<'s> {
    kind: NodeKind,
    source: &'s str,
}

impl<'s> Node<'s> {
    pub fn new(kind: NodeKind, source: &'s str) -> Self {
        Self { kind, source }
    }
}

#[derive(Debug)]
pub(crate) enum NodeKind {
    Scalar(ScalarKind),
    Array,
    Object,
    Field,
    Key(StringKind),
}

#[derive(Debug)]
pub(crate) enum ScalarKind {
    Null,
    Bool(bool),
    Number,
    String(StringKind),
}

#[derive(Debug)]
pub(crate) enum StringKind {
    Plain,
    Escaped,
}
