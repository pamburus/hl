use super::*;

#[test]
fn test_insert() {
    let mut set = KeySet::default();
    assert!(set.insert("a"));
    assert!(set.insert("b"));
    assert!(!set.insert("a"));
    assert!(set.contains("b"));
    assert!(!set.contains("c"));
    assert!(set.insert(""));
    assert!(!set.insert(""));
}

#[test]
fn test_equal_hashes() {
    // Keys with the same hash but different text are distinct.
    let mut set = KeySet::default();
    assert!(set.insert_hashed(1, "a"));
    assert!(set.insert_hashed(1, "b"));
    assert!(!set.insert_hashed(1, "a"));
    assert!(set.contains_hashed(1, "b"));
    assert!(!set.contains_hashed(1, "c"));
}

#[test]
fn test_spill() {
    let keys = (0..100).map(|i| format!("key{i}")).collect::<Vec<_>>();
    let mut set = KeySet::default();
    for key in &keys {
        assert!(set.insert(key));
    }
    for key in &keys {
        assert!(!set.insert(key));
    }
}
