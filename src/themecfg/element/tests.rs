use super::super::tests::theme;
use super::Element;
use crate::themecfg::{Color, PlainColor};

#[test]
fn test_element_names_case_sensitive() {
    let theme = theme("v0-invalid-element-case");

    let message = theme.elements.get(&Element::Message);
    assert!(message.is_some(), "Element 'message' (lowercase) should be loaded");
    assert_eq!(
        message.unwrap().foreground,
        Some(Color::Plain(PlainColor::Green)),
        "Valid 'message' element should have green foreground"
    );
}

#[test]
fn test_element_parent_queries() {
    let pairs = Element::nested();
    assert_ne!(pairs.len(), 0);
    assert!(pairs.contains(&(Element::Level, Element::LevelInner)));
}

#[test]
fn test_element_is_inner() {
    assert!(Element::LevelInner.is_inner());
    assert!(Element::InputNumber.is_inner());
    assert!(!Element::Level.is_inner());
}
