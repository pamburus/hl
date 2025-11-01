use super::*;

#[test]
fn test_storage() {
    let mut storage = Vec::new();
    assert_eq!(storage.len(), 0);
    assert!(storage.is_empty());
    storage.push(1);
    assert_eq!(storage.len(), 1);
    assert!(!storage.is_empty());
    assert_eq!(storage.first(), Some(&1));
    assert_eq!(storage.get_mut(0), Some(&mut 1));
    storage.clear();
    assert_eq!(storage.len(), 0);
    assert!(storage.is_empty());
}
