use super::*;

use std::{fs, path::PathBuf};

pub(super) fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("hl-fileid-{}-{}", std::process::id(), name))
}

#[test]
fn test_file_id_is_stable_for_same_file() {
    let path = temp_path("stable");
    fs::write(&path, b"a\n").unwrap();
    let before = fs::metadata(&path).unwrap().file_id();
    fs::write(&path, b"a\nb\n").unwrap();
    let after = fs::metadata(&path).unwrap().file_id();
    fs::remove_file(&path).unwrap();

    assert_eq!(before, after);
}
