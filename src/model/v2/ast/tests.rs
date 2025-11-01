use super::*;

#[test]
fn test_builder() {
    let mut container = Container::new();
    let root = container.metaroot();
    root.add_scalar(Scalar::Bool(true))
        .add_composite(Composite::Array, |b| (b.add_scalar(Scalar::Bool(false)), Ok(())))
        .1
        .unwrap();
    assert_eq!(container.roots().len(), 2);
}

#[test]
fn test_builder_attach() {
    let mut container = Container::new();
    let root = container.metaroot();
    let attachment = root
        .add_scalar(Scalar::Bool(true))
        .attach("attachment")
        .add_composite(Composite::Array, |b| {
            let (b, attachment) = b.detach();
            assert_eq!(attachment, "attachment");
            (b.add_scalar(Scalar::Bool(false)).attach("another attachment"), Ok(()))
        })
        .0
        .detach()
        .1;
    assert_eq!(container.roots().len(), 2);
    assert_eq!(attachment, "another attachment");
}
