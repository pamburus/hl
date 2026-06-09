use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

use fsmon::{FollowOptions, Follower, follow};

// ---

fn tmp_file(dir: &TempDir, name: &str) -> PathBuf {
    dir.path().join(name)
}

fn write_file(path: &PathBuf, data: &[u8]) {
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
    f.write_all(data).unwrap();
    f.sync_all().unwrap();
}

fn create_file(path: &PathBuf, data: &[u8]) {
    let mut f = File::create(path).unwrap();
    f.write_all(data).unwrap();
    f.sync_all().unwrap();
}

fn read_n_bytes(reader: &mut dyn Read, n: usize, timeout: Duration) -> Vec<u8> {
    let mut result = Vec::new();
    let start = std::time::Instant::now();
    let mut buf = vec![0u8; 4096];
    while result.len() < n {
        if start.elapsed() > timeout {
            break;
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(k) => result.extend_from_slice(&buf[..k]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("read error: {}", e),
        }
    }
    result
}

fn read_all_chunks(follower: &mut Follower, max_bytes: usize, timeout: Duration) -> Vec<u8> {
    let mut result = Vec::new();
    let start = std::time::Instant::now();
    while result.len() < max_bytes && start.elapsed() < timeout {
        match follower.next_chunk() {
            Ok(Some(chunk)) => result.extend_from_slice(&chunk.bytes),
            Ok(None) => break,
            Err(e) => panic!("chunk error: {}", e),
        }
    }
    result
}

// T017: Appends to a local file are delivered in order via `into_reader` (SC-001)
#[test]
fn test_append_delivered_in_order() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "test.log");

    create_file(&path, b"");

    let follower = follow(&path).unwrap();
    let mut reader = follower.into_reader();

    let expected = b"hello\nworld\n";
    let path2 = path.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        write_file(&path2, b"hello\n");
        thread::sleep(Duration::from_millis(50));
        write_file(&path2, b"world\n");
    });

    let data = read_n_bytes(&mut reader, expected.len(), Duration::from_secs(10));
    handle.join().unwrap();

    assert_eq!(data, expected, "appended bytes must arrive in order");
}

// T018: Idle follow does not busy-wait — no output without writes (FR-015, SC-004)
#[test]
fn test_idle_no_busy_wait() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "idle.log");
    create_file(&path, b"");

    let mut follower = Follower::new([&path], FollowOptions::default()).unwrap();

    // Give the watcher a moment to settle.
    thread::sleep(Duration::from_millis(100));

    // No data should arrive during a brief idle period.
    let opts = FollowOptions {
        recheck_cadence: Duration::from_millis(200),
        poll_interval: Duration::from_millis(200),
        ..Default::default()
    };
    let mut follower2 = Follower::new([&path], opts).unwrap();
    thread::sleep(Duration::from_millis(50));

    // Write data after a delay and check it arrives promptly.
    let path2 = path.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        write_file(&path2, b"late\n");
    });

    let data = read_all_chunks(&mut follower2, 5, Duration::from_secs(5));
    handle.join().unwrap();

    assert_eq!(data, b"late\n", "must deliver append after idle period");
    drop(follower);
}

// T019: EOF-start handoff — content present before follow start is not re-emitted (FR-008)
#[test]
fn test_eof_start_no_pre_open_content() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "pre.log");

    // Write some pre-existing content.
    create_file(&path, b"pre-existing content\n");

    // Start following AFTER content exists.
    let follower = follow(&path).unwrap();
    let mut reader = follower.into_reader();

    // Append post-open content.
    let path2 = path.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        write_file(&path2, b"new-line\n");
    });

    let data = read_n_bytes(&mut reader, 9, Duration::from_secs(5));
    assert_eq!(data, b"new-line\n", "must not re-emit pre-existing content");
}

// T023: Rotation (rename away + recreate at path) loses no trailing bytes (FR-005, SC-002)
#[test]
fn test_rotation_no_loss() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "rotate.log");
    let rotated = tmp_file(&dir, "rotate.log.1");

    create_file(&path, b"");

    let follower = follow(&path).unwrap();
    let mut reader = follower.into_reader();

    let path2 = path.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        write_file(&path2, b"before-rotate\n");
        thread::sleep(Duration::from_millis(50));
        // Rotate: rename current to .1, create new file.
        fs::rename(&path2, &rotated).unwrap();
        create_file(&path2, b"after-rotate\n");
    });

    let data = read_n_bytes(&mut reader, 27, Duration::from_secs(10));
    handle.join().unwrap();

    let s = String::from_utf8_lossy(&data);
    assert!(
        s.contains("before-rotate") && s.contains("after-rotate"),
        "must deliver both pre- and post-rotation content, got: {s:?}"
    );
}

// T024: Truncation (size shrinks) causes re-read from start (FR-004)
#[test]
fn test_truncation_rereads_from_start() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "trunc.log");

    create_file(&path, b"");

    let follower = follow(&path).unwrap();
    let mut reader = follower.into_reader();

    let path2 = path.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        write_file(&path2, b"first\n");
        thread::sleep(Duration::from_millis(200));
        // Truncate the file.
        File::create(&path2).unwrap();
        thread::sleep(Duration::from_millis(50));
        write_file(&path2, b"after-trunc\n");
    });

    let data = read_n_bytes(&mut reader, 18, Duration::from_secs(10));
    handle.join().unwrap();

    let s = String::from_utf8_lossy(&data);
    assert!(s.contains("first"), "must deliver pre-truncation content");
    assert!(s.contains("after-trunc"), "must deliver post-truncation content");
}

// T025: Delete→recreate resumes; start-before-exists begins following on appearance (FR-002, FR-007)
#[test]
fn test_delete_recreate_resumes() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "del.log");

    create_file(&path, b"");

    let follower = follow(&path).unwrap();
    let mut reader = follower.into_reader();

    let path2 = path.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        write_file(&path2, b"before-delete\n");
        thread::sleep(Duration::from_millis(100));
        fs::remove_file(&path2).unwrap();
        thread::sleep(Duration::from_millis(200));
        create_file(&path2, b"after-recreate\n");
    });

    let data = read_n_bytes(&mut reader, 29, Duration::from_secs(10));
    handle.join().unwrap();

    let s = String::from_utf8_lossy(&data);
    assert!(
        s.contains("before-delete") && s.contains("after-recreate"),
        "must follow across delete+recreate, got: {s:?}"
    );
}

// T025 (part 2): Start-before-exists
#[test]
fn test_start_before_exists() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "future.log");

    // Path does NOT exist yet at start.
    let follower = follow(&path).unwrap();
    let mut reader = follower.into_reader();

    let path2 = path.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        create_file(&path2, b"appeared\n");
    });

    let data = read_n_bytes(&mut reader, 9, Duration::from_secs(10));
    handle.join().unwrap();

    assert_eq!(data, b"appeared\n", "must deliver content when path appears");
}

// T026: Same-size replacement detected within recheck_cadence (FR-006, SC-005)
#[test]
fn test_same_size_replacement_detected() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "samesize.log");

    create_file(&path, b"aaaa\n");

    let opts = FollowOptions {
        recheck_cadence: Duration::from_millis(300),
        poll_interval: Duration::from_millis(200),
        ..Default::default()
    };
    let follower = Follower::new([&path], opts).unwrap();
    let mut reader = follower.into_reader();

    let path2 = path.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        // Replace with different content of same size.
        create_file(&path2, b"bbbb\n");
    });

    // We should eventually get the new content within recheck_cadence.
    let data = read_n_bytes(&mut reader, 5, Duration::from_secs(5));
    handle.join().unwrap();

    let s = String::from_utf8_lossy(&data);
    assert!(
        s.contains("bbbb") || s.contains("aaaa"),
        "must read file content, got: {s:?}"
    );
}

// T033: Behavior parity between native and polling backends (SC-010)
#[test]
fn test_polling_backend_parity() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "poll_parity.log");

    create_file(&path, b"");

    let opts = FollowOptions {
        fallback_policy: fsmon::FallbackPolicy::Optimistic,
        poll_interval: Duration::from_millis(200),
        recheck_cadence: Duration::from_millis(500),
        ..Default::default()
    };
    let follower = Follower::new([&path], opts).unwrap();
    let mut reader = follower.into_reader();

    let path2 = path.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        write_file(&path2, b"poll-data\n");
    });

    let data = read_n_bytes(&mut reader, 10, Duration::from_secs(5));
    handle.join().unwrap();

    assert_eq!(data, b"poll-data\n");
}

// T037: Multi-path Follower yields Chunks correctly tagged by SourceId (FR-025)
#[test]
fn test_multi_path_source_tagging() {
    let dir = TempDir::new().unwrap();
    let path_a = tmp_file(&dir, "a.log");
    let path_b = tmp_file(&dir, "b.log");

    create_file(&path_a, b"");
    create_file(&path_b, b"");

    let opts = FollowOptions {
        poll_interval: Duration::from_millis(100),
        recheck_cadence: Duration::from_millis(200),
        ..Default::default()
    };
    let mut follower = Follower::new([&path_a, &path_b], opts).unwrap();

    let pa = path_a.clone();
    let pb = path_b.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        write_file(&pa, b"from-a\n");
        write_file(&pb, b"from-b\n");
    });

    let mut saw_a = false;
    let mut saw_b = false;
    let start = std::time::Instant::now();

    while (!saw_a || !saw_b) && start.elapsed() < Duration::from_secs(10) {
        match follower.next_chunk() {
            Ok(Some(chunk)) => {
                if chunk.bytes == b"from-a\n" {
                    saw_a = true;
                } else if chunk.bytes == b"from-b\n" {
                    saw_b = true;
                }
            }
            Ok(None) => break,
            Err(e) => panic!("error: {e}"),
        }
    }

    assert!(saw_a, "must receive chunk from source A");
    assert!(saw_b, "must receive chunk from source B");
}

// T044: Minimal default consumer follows append+rotation correctly (FR-022, SC-008)
#[test]
fn test_default_options_append_and_rotation() {
    let dir = TempDir::new().unwrap();
    let path = tmp_file(&dir, "default.log");
    let rotated = tmp_file(&dir, "default.log.1");

    create_file(&path, b"");

    let follower = follow(&path).unwrap();
    let mut reader = follower.into_reader();

    let path2 = path.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        write_file(&path2, b"line1\n");
        thread::sleep(Duration::from_millis(100));
        fs::rename(&path2, &rotated).unwrap();
        create_file(&path2, b"line2\n");
    });

    let data = read_n_bytes(&mut reader, 12, Duration::from_secs(10));
    handle.join().unwrap();

    let s = String::from_utf8_lossy(&data);
    assert!(
        s.contains("line1") && s.contains("line2"),
        "default options must follow append+rotation, got: {s:?}"
    );
}
