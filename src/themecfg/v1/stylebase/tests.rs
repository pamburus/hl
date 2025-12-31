use super::super::super::{Role, StyleBase};

#[test]
fn test_v1_style_base_construction() {
    let single = StyleBase::from(Role::Warning);
    assert_eq!(single.len(), 1);
    assert_eq!(single[0], Role::Warning);

    let multiple = StyleBase::from(vec![Role::Primary, Role::Secondary, Role::Warning]);
    assert_eq!(multiple.len(), 3);
    assert_eq!(multiple[0], Role::Primary);
    assert_eq!(multiple[1], Role::Secondary);
    assert_eq!(multiple[2], Role::Warning);

    let empty = StyleBase::default();
    assert!(empty.is_empty());
    assert!(!single.is_empty());
    assert!(!multiple.is_empty());
}
