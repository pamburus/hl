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
    assert!(result);

    let roots = collect(tree.roots());
    assert_eq!(roots, [1, 2]);
}
