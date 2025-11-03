use super::*;
use encstr::EncodedString;
use rstest::rstest;

// Container Tests

#[test]
fn test_container_new() {
    let container = Container::new();
    assert_eq!(container.roots().len(), 0);
}

#[test]
fn test_container_default() {
    let container: Container = Default::default();
    assert_eq!(container.roots().len(), 0);
}

#[test]
fn test_container_clear() {
    let mut container = Container::new();
    let root = container.metaroot();
    root.add_scalar(Scalar::Bool(true));
    assert_eq!(container.roots().len(), 1);

    container.clear();
    assert_eq!(container.roots().len(), 0);
}

#[test]
fn test_container_reserve() {
    let mut container = Container::new();
    container.reserve(100);

    let root = container.metaroot();
    root.add_scalar(Scalar::Bool(true)).add_scalar(Scalar::Bool(false));

    assert_eq!(container.roots().len(), 2);
}

#[test]
fn test_container_roots_and_nodes() {
    let mut container = Container::new();
    let root = container.metaroot();
    root.add_scalar(Scalar::Bool(true));

    assert_eq!(container.roots().len(), 1);
    assert_eq!(container.nodes().len(), 1);
}

// Builder Tests

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

#[test]
fn test_builder_multiple_scalars() {
    let mut container = Container::new();
    let root = container.metaroot();
    root.add_scalar(Scalar::Bool(true))
        .add_scalar(Scalar::Bool(false))
        .add_scalar(Scalar::Null);

    assert_eq!(container.roots().len(), 3);
}

#[test]
fn test_builder_nested_composites() {
    let mut container = Container::new();
    let root = container.metaroot();
    root.add_composite(Composite::Object, |b| {
        (
            b.add_composite(Composite::Array, |b| (b.add_scalar(Scalar::Bool(true)), Ok(())))
                .0,
            Ok(()),
        )
    });

    assert_eq!(container.roots().len(), 1);
}

#[test]
fn test_builder_composite_error_propagation() {
    let mut container = Container::new();
    let root = container.metaroot();
    let (_, result) = root.add_composite(Composite::Array, |b| {
        (b.add_scalar(Scalar::Bool(true)), Err(("error", 0..5)))
    });

    assert!(result.is_err());
}

#[test]
fn test_builder_composite_with_multiple_children() {
    let mut container = Container::new();
    let root = container.metaroot();
    let result = root.add_composite(Composite::Object, |b| {
        (
            b.add_scalar(Scalar::Bool(true))
                .add_scalar(Scalar::Null)
                .add_scalar(Scalar::Bool(false)),
            Ok(()),
        )
    });
    assert!(result.1.is_ok());
    assert_eq!(container.roots().len(), 1);
}

#[test]
fn test_builder_checkpoint() {
    let mut container = Container::new();
    let root = container.metaroot();
    let checkpoint = root.checkpoint();

    let root_after = root.add_scalar(Scalar::Bool(true));
    let index = root_after.first_node_index(&checkpoint);

    assert!(index.unfold().is_some());
}

// Discarder Tests

#[test]
fn test_discarder_default() {
    let _discarder: Discarder = Discarder::default();
}

#[test]
fn test_discarder_add_scalar() {
    let discarder = Discarder::default();
    let discarder = discarder.add_scalar(Scalar::Bool(true));
    let discarder = discarder.add_scalar(Scalar::Null);
    assert!(true);
}

#[test]
fn test_discarder_add_composite() {
    let discarder = Discarder::default();
    let (discarder, result) =
        discarder.add_composite(Composite::Array, |b| (b.add_scalar(Scalar::Bool(false)), Ok(())));

    assert!(result.is_ok());
}

#[test]
fn test_discarder_checkpoint() {
    let discarder = Discarder::default();
    let checkpoint = discarder.checkpoint();
    assert_eq!(checkpoint, ());
}

#[test]
fn test_discarder_first_node_index() {
    let discarder = Discarder::default();
    let checkpoint = discarder.checkpoint();
    let index = discarder.first_node_index(&checkpoint);

    assert!(index.unfold().is_none());
}

#[test]
fn test_discarder_attach_and_detach() {
    let discarder = Discarder::default();
    let discarder_with_attachment = discarder.attach("test_attachment");
    let (discarder_without, attachment_value) = discarder_with_attachment.detach();

    assert_eq!(attachment_value, "test_attachment");
}

#[test]
fn test_discarder_attach_multiple() {
    let discarder = Discarder::default();
    let attached1 = discarder.attach("first");
    let attached2 = attached1.attach("second");
    let (_, final_attachment) = attached2.detach();

    assert_eq!(final_attachment, "second");
}

#[test]
fn test_discarder_composite_with_attachment() {
    let discarder = Discarder::default();
    let discarder_attached = discarder.attach("attachment_value");
    let (discarder_with_composite, _) =
        discarder_attached.add_composite(Composite::Array, |b| (b.add_scalar(Scalar::Bool(true)), Ok(())));

    let (_, final_attachment) = discarder_with_composite.detach();
    assert_eq!(final_attachment, "attachment_value");
}

// Value Factory Methods

#[test]
fn test_value_null() {
    let value = Value::null();
    assert!(matches!(value, Value::Scalar(Scalar::Null)));
}

#[test]
fn test_value_bool_true() {
    let value = Value::bool(true);
    assert!(matches!(value, Value::Scalar(Scalar::Bool(true))));
}

#[test]
fn test_value_bool_false() {
    let value = Value::bool(false);
    assert!(matches!(value, Value::Scalar(Scalar::Bool(false))));
}

#[test]
fn test_value_number() {
    let number_str = "123.45";
    let value = Value::number(number_str);
    assert!(matches!(value, Value::Scalar(Scalar::Number(n)) if n == number_str));
}

#[test]
fn test_value_number_negative() {
    let number_str = "-999";
    let value = Value::number(number_str);
    assert!(matches!(value, Value::Scalar(Scalar::Number(n)) if n == number_str));
}

#[test]
fn test_value_string() {
    let encoded = EncodedString::raw("hello");
    let value = Value::string(encoded);
    assert!(matches!(value, Value::Scalar(Scalar::String(_))));
}

#[test]
fn test_value_array() {
    let value = Value::array();
    assert!(matches!(value, Value::Composite(Composite::Array)));
}

#[test]
fn test_value_object() {
    let value = Value::object();
    assert!(matches!(value, Value::Composite(Composite::Object)));
}

#[test]
fn test_value_field() {
    let field_name = EncodedString::raw("name");
    let value = Value::field(field_name);
    assert!(matches!(value, Value::Composite(Composite::Field(_))));
}

// Value From Implementation

#[test]
fn test_value_from_scalar_null() {
    let scalar = Scalar::Null;
    let value: Value = scalar.into();
    assert!(matches!(value, Value::Scalar(Scalar::Null)));
}

#[test]
fn test_value_from_scalar_bool() {
    let scalar = Scalar::Bool(true);
    let value: Value = scalar.into();
    assert!(matches!(value, Value::Scalar(Scalar::Bool(true))));
}

#[test]
fn test_value_from_scalar_number() {
    let scalar = Scalar::Number("42");
    let value: Value = scalar.into();
    assert!(matches!(value, Value::Scalar(Scalar::Number("42"))));
}

#[test]
fn test_value_from_scalar_string() {
    let encoded = EncodedString::raw("test");
    let scalar = Scalar::String(encoded);
    let value: Value = scalar.into();
    assert!(matches!(value, Value::Scalar(Scalar::String(_))));
}

#[test]
fn test_value_from_composite_array() {
    let composite = Composite::Array;
    let value: Value = composite.into();
    assert!(matches!(value, Value::Composite(Composite::Array)));
}

#[test]
fn test_value_from_composite_object() {
    let composite = Composite::Object;
    let value: Value = composite.into();
    assert!(matches!(value, Value::Composite(Composite::Object)));
}

#[test]
fn test_value_from_composite_field() {
    let encoded = EncodedString::raw("field");
    let composite = Composite::Field(encoded);
    let value: Value = composite.into();
    assert!(matches!(value, Value::Composite(Composite::Field(_))));
}

// Scalar::as_text() with rstest

#[rstest]
#[case(Scalar::Null, "null")]
#[case(Scalar::Bool(true), "true")]
#[case(Scalar::Bool(false), "false")]
fn test_scalar_as_text_special_values(#[case] scalar: Scalar, #[case] expected: &str) {
    let text = scalar.as_text();
    assert_eq!(text.source(), expected);
}

#[test]
fn test_scalar_as_text_number() {
    let scalar = Scalar::Number("42");
    let text = scalar.as_text();
    assert_eq!(text.source(), "42");
}

#[test]
fn test_scalar_as_text_number_float() {
    let scalar = Scalar::Number("3.14159");
    let text = scalar.as_text();
    assert_eq!(text.source(), "3.14159");
}

#[test]
fn test_scalar_as_text_number_negative() {
    let scalar = Scalar::Number("-999");
    let text = scalar.as_text();
    assert_eq!(text.source(), "-999");
}

#[test]
fn test_scalar_as_text_number_zero() {
    let scalar = Scalar::Number("0");
    let text = scalar.as_text();
    assert_eq!(text.source(), "0");
}

#[test]
fn test_scalar_as_text_string() {
    let string_encoded = EncodedString::raw("hello world");
    let scalar = Scalar::String(string_encoded);
    let text = scalar.as_text();
    assert_eq!(text.source(), "hello world");
}

#[test]
fn test_scalar_as_text_empty_string() {
    let string_encoded = EncodedString::raw("");
    let scalar = Scalar::String(string_encoded);
    let text = scalar.as_text();
    assert_eq!(text.source(), "");
}

// Scalar Variants

#[test]
fn test_scalar_null_variant() {
    match Scalar::Null {
        Scalar::Null => assert!(true),
        _ => panic!("Expected Scalar::Null"),
    }
}

#[test]
fn test_scalar_bool_true_variant() {
    match Scalar::Bool(true) {
        Scalar::Bool(true) => assert!(true),
        _ => panic!("Expected Scalar::Bool(true)"),
    }
}

#[test]
fn test_scalar_bool_false_variant() {
    match Scalar::Bool(false) {
        Scalar::Bool(false) => assert!(true),
        _ => panic!("Expected Scalar::Bool(false)"),
    }
}

#[test]
fn test_scalar_number_variant() {
    let num = "123";
    match Scalar::Number(num) {
        Scalar::Number(n) => assert_eq!(n, num),
        _ => panic!("Expected Scalar::Number"),
    }
}

#[test]
fn test_scalar_string_variant() {
    let encoded = EncodedString::raw("text");
    match Scalar::String(encoded) {
        Scalar::String(_) => assert!(true),
        _ => panic!("Expected Scalar::String"),
    }
}

// Composite Variants

#[test]
fn test_composite_array_variant() {
    match Composite::Array {
        Composite::Array => assert!(true),
        _ => panic!("Expected Composite::Array"),
    }
}

#[test]
fn test_composite_object_variant() {
    match Composite::Object {
        Composite::Object => assert!(true),
        _ => panic!("Expected Composite::Object"),
    }
}

#[test]
fn test_composite_field_variant() {
    let field = EncodedString::raw("test_field");
    match Composite::Field(field) {
        Composite::Field(_) => assert!(true),
        _ => panic!("Expected Composite::Field"),
    }
}

// Integration Tests

#[test]
fn test_builder_with_field_and_values() {
    let mut container = Container::new();
    let root = container.metaroot();
    root.add_composite(Composite::Object, |b| {
        (
            b.add_composite(Composite::Field(EncodedString::raw("id")), |b| {
                (b.add_scalar(Scalar::Number("123")), Ok(()))
            })
            .0,
            Ok(()),
        )
    });

    assert_eq!(container.roots().len(), 1);
}

#[test]
fn test_builder_attach_with_nested_composite() {
    let mut container = Container::new();
    let root = container.metaroot();
    let context = root
        .attach("context_1")
        .add_composite(Composite::Object, |b| (b.add_scalar(Scalar::Bool(true)), Ok(())))
        .0
        .detach()
        .1;

    assert_eq!(context, "context_1");
}

#[test]
fn test_multiple_roots_with_different_types() {
    let mut container = Container::new();
    let root = container.metaroot();

    root.add_scalar(Scalar::Null)
        .add_scalar(Scalar::Bool(true))
        .add_composite(Composite::Array, |b| (b.add_scalar(Scalar::Number("0")), Ok(())))
        .0
        .add_composite(Composite::Object, |b| {
            (
                b.add_composite(Composite::Field(EncodedString::raw("key")), |b| {
                    (b.add_scalar(Scalar::String(EncodedString::raw("value"))), Ok(()))
                })
                .0,
                Ok(()),
            )
        });

    assert!(container.roots().len() > 2);
}

#[test]
fn test_container_operations_sequence() {
    let mut container = Container::new();
    assert_eq!(container.roots().len(), 0);

    container.reserve(50);

    let root = container.metaroot();
    root.add_scalar(Scalar::Bool(true));
    assert_eq!(container.roots().len(), 1);

    container.clear();
    assert_eq!(container.roots().len(), 0);
}

#[test]
fn test_scalar_all_number_variants_as_text() {
    let test_numbers = vec!["0", "1", "-42", "3.14", "1e10"];

    for num_str in test_numbers {
        let scalar = Scalar::Number(num_str);
        let text = scalar.as_text();
        assert_eq!(text.source(), num_str);
    }
}

// HeapOptStorage Tests

#[test]
fn test_heapoptstorage_works_as_storage() {
    let mut tree: flat_tree::FlatTree<i32, HeapOptStorage<i32>> = flat_tree::FlatTree::new();
    tree.push(1);
    tree.push(2);
    tree.push(3);
    assert_eq!(tree.nodes().len(), 3);
}

#[test]
fn test_heapoptstorage_clear() {
    let mut tree: flat_tree::FlatTree<i32, HeapOptStorage<i32>> = flat_tree::FlatTree::new();
    tree.push(1);
    tree.push(2);
    assert_eq!(tree.nodes().len(), 2);
    
    tree.clear();
    assert_eq!(tree.nodes().len(), 0);
}

#[test]
fn test_heapoptstorage_reserve() {
    let mut tree: flat_tree::FlatTree<i32, HeapOptStorage<i32>> = flat_tree::FlatTree::new();
    tree.reserve(100);
    
    for i in 0..50 {
        tree.push(i);
    }
    assert_eq!(tree.nodes().len(), 50);
}

#[test]
fn test_heapoptstorage_get() {
    use flat_tree::Storage;
    
    let mut tree: flat_tree::FlatTree<i32, HeapOptStorage<i32>> = flat_tree::FlatTree::new();
    tree.push(42);
    tree.push(99);
    
    let storage = tree.storage();
    assert!(storage.get(0).is_some());
    assert!(storage.get(1).is_some());
    assert!(storage.get(2).is_none());
}

#[test]
fn test_heapoptstorage_get_mut() {
    use flat_tree::Storage;
    
    let mut tree: flat_tree::FlatTree<i32, HeapOptStorage<i32>> = flat_tree::FlatTree::new();
    tree.push(10);
    tree.push(20);
    
    // get_mut is called internally by FlatTree when building
    tree.build(30, |builder| {
        (builder.push(40), Ok::<(), (&str, std::ops::Range<usize>)>(()))
    });
    
    assert_eq!(tree.nodes().len(), 4);
}
