//
use flat_tree::FlatTree;

pub fn parse<'s>(source: &'s str) -> Container<'s> {
    let tree = FlatTree::default();
    let mut container = Container { tree };
    let root = container.tree.root();
    parse_node(&mut container, root, source);
    container
}

pub struct Container<'s> {
    tree: FlatTree<Node<'s>>,
}

struct Node<'s> {
    kind: NodeKind,
    source: &'s str,
}

enum NodeKind {
    Scalar(ScalarKind),
    Array,
    Object,
    Field,
    Key(StringKind),
}

enum ScalarKind {
    Null,
    Bool(bool),
    Number,
    String(StringKind),
}

enum StringKind {
    Plain,
    Escaped,
}
