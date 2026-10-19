use std::fs;

use crate::fileid::{MetadataExt as _, tests::temp_path};

#[test]
fn test_file_id_differs_for_replaced_file() {
    let path = temp_path("replaced");
    let new = temp_path("replaced-new");
    // Keep the old file alive under another name, so its identity cannot be reused.
    let old = temp_path("replaced-old");
    fs::write(&path, b"old\n").unwrap();
    fs::write(&new, b"new\n").unwrap();
    let before = fs::metadata(&path).unwrap().file_id();
    fs::rename(&path, &old).unwrap();
    fs::rename(&new, &path).unwrap();
    let after = fs::metadata(&path).unwrap().file_id();
    fs::remove_file(&path).unwrap();
    fs::remove_file(&old).unwrap();

    assert!(before.is_some());
    assert_ne!(before, after);
}
