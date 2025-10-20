use super::*;

#[test]
fn test_filesystem() {
    let fs = mem::FileSystem::new();
    let path = Path::new("file.txt");

    assert!(!fs.exists(path).unwrap());

    let mut file = fs.create(path).unwrap();
    file.write_all(b"hello world").unwrap();
    file.flush().unwrap();

    let res = fs.create(path);
    assert!(res.is_err());
    assert_eq!(res.err().map(|e| e.kind()), Some(io::ErrorKind::AlreadyExists));

    assert!(fs.exists(path).unwrap());

    let meta = fs.metadata(path).unwrap();
    assert_eq!(meta.len, 11);

    let res = fs.metadata(Path::new("nonexistent.txt"));
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err().kind(), io::ErrorKind::NotFound));

    let canonical_path = fs.canonicalize(path).unwrap();
    assert_eq!(canonical_path, PathBuf::from("/tmp/file.txt"));

    let mut file = fs.open(path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();

    assert_eq!(buf, b"hello world");

    let meta = file.metadata().unwrap();
    assert_eq!(meta.len, 11);

    let res = fs.open(Path::new("nonexistent.txt"));
    assert!(res.is_err());
    assert_eq!(res.err().map(|e| e.kind()), Some(io::ErrorKind::NotFound));
}

#[test]
fn test_filesystem_reference() {
    let fs = mem::FileSystem::new();
    let fs_ref = &fs;
    let path = Path::new("ref_test.txt");

    // Test all methods through reference
    assert!(!fs_ref.exists(path).unwrap());

    let mut file = fs_ref.create(path).unwrap();
    file.write_all(b"reference test").unwrap();
    file.flush().unwrap();

    assert!(fs_ref.exists(path).unwrap());

    let meta = fs_ref.metadata(path).unwrap();
    assert_eq!(meta.len, 14);

    let canonical_path = fs_ref.canonicalize(path).unwrap();
    assert_eq!(canonical_path, PathBuf::from("/tmp/ref_test.txt"));

    let mut file = fs_ref.open(path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"reference test");
}

#[test]
fn test_filesystem_arc() {
    let fs = std::sync::Arc::new(mem::FileSystem::new());
    let path = Path::new("arc_test.txt");

    // Test all methods through Arc
    assert!(!fs.exists(path).unwrap());

    let mut file = fs.create(path).unwrap();
    file.write_all(b"arc test").unwrap();
    file.flush().unwrap();

    assert!(fs.exists(path).unwrap());

    let meta = fs.metadata(path).unwrap();
    assert_eq!(meta.len, 8);

    let canonical_path = fs.canonicalize(path).unwrap();
    assert_eq!(canonical_path, PathBuf::from("/tmp/arc_test.txt"));

    let mut file = fs.open(path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"arc test");
}
