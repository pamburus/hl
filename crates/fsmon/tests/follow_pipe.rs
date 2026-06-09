use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use fsmon::{FollowOptions, Follower};

// T052: Directory and special-file rejection (FR-026)
#[test]
fn test_directory_rejected() {
    let dir = TempDir::new().unwrap();
    let result = Follower::new([dir.path()], FollowOptions::default());
    assert!(result.is_err(), "directories must be rejected");
    if let Err(e) = result {
        let s = e.to_string();
        assert!(
            s.contains("directory") || s.contains("unsupported"),
            "error message should mention directory: {s}"
        );
    }
}

// T052: FIFO (named pipe) is accepted as a stream (FR-026)
#[cfg(unix)]
#[test]
fn test_fifo_accepted() {
    use std::io::{Read, Write};
    use std::thread;
    use std::time::Duration;

    let dir = TempDir::new().unwrap();
    let fifo_path: PathBuf = dir.path().join("test.fifo");

    // Create a named pipe.
    unsafe {
        let path_cstr = std::ffi::CString::new(fifo_path.to_str().unwrap()).unwrap();
        libc::mkfifo(path_cstr.as_ptr(), 0o600);
    }

    let fifo_path2 = fifo_path.clone();
    // Writer thread: write data to the FIFO then close it.
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        let mut f = fs::OpenOptions::new().write(true).open(&fifo_path2).unwrap();
        f.write_all(b"pipe-data\n").unwrap();
    });

    let follower = fsmon::follow(&fifo_path).unwrap();
    let mut reader = follower.into_reader();

    let mut buf = vec![0u8; 64];
    let mut received = Vec::new();
    let start = std::time::Instant::now();
    while received.len() < 10 && start.elapsed() < Duration::from_secs(5) {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => received.extend_from_slice(&buf[..n]),
            Err(e) => panic!("read error: {e}"),
        }
    }
    handle.join().unwrap();

    assert_eq!(received, b"pipe-data\n");
}
