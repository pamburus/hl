// Integration tests targeting production code paths that the primary suites
// leave uncovered on the dev platform: the core `Watcher` API, multi-source
// `next_chunk`, in-place truncation re-read, and the rotation drain path.
// (Lives under tests/ so it is excluded from coverage as test code.)

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use fsmon::watch::{Event, Watcher};
use fsmon::{Engine, FollowOptions, Follower};

fn create_file(path: &PathBuf, data: &[u8]) {
    let mut f = fs::File::create(path).unwrap();
    f.write_all(data).unwrap();
    f.sync_all().unwrap();
}

fn append_file(path: &PathBuf, data: &[u8]) {
    let mut f = fs::OpenOptions::new().append(true).open(path).unwrap();
    f.write_all(data).unwrap();
    f.sync_all().unwrap();
}

/// Drain `next_chunk` until `needle` is seen or `timeout` elapses.
fn drain_until(f: &mut Follower, needle: &str, timeout: Duration) -> Vec<u8> {
    let start = Instant::now();
    let mut acc = Vec::new();
    while start.elapsed() < timeout {
        match f.next_chunk() {
            Ok(Some(chunk)) => acc.extend_from_slice(&chunk.bytes),
            Ok(None) => break,
            Err(_) => break,
        }
        if String::from_utf8_lossy(&acc).contains(needle) {
            break;
        }
    }
    acc
}

// classify(): a path containing an interior NUL cannot become a CString and is
// treated conservatively as NotConfirmed.
#[cfg(unix)]
#[test]
fn classify_rejects_path_with_interior_nul() {
    use fsmon::classify::{Reliability, classify};
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    let p = Path::new(OsStr::from_bytes(b"/tmp/with\0nul"));
    assert_eq!(classify(p), Reliability::NotConfirmed);
}

// Core Watcher API: recv() surfaces a data event for a live append, and
// engine() reports the per-path engine for the woken source.
#[test]
fn watcher_core_api_reports_event_and_engine() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("watched.log");
    create_file(&path, b"start\n");

    let mut watcher = Watcher::new([&path], FollowOptions::default()).unwrap();

    // Append from another thread so recv() has an event to deliver.
    let p = path.clone();
    thread::spawn(move || {
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(100));
            let _ = append_file(&p, b"more\n");
        }
    });

    // Run recv() off-thread so the test can never hang.
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        loop {
            match watcher.recv() {
                Ok(Event::DataAvailable(s)) => {
                    let _ = tx.send(Some(watcher.engine(s)));
                    return;
                }
                Ok(_) => continue,
                Err(_) => {
                    let _ = tx.send(None);
                    return;
                }
            }
        }
    });

    let engine = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("no event within timeout");
    // A local temp file is followed natively; classification may still fall
    // back to polling on exotic CI filesystems, so accept either engine.
    assert!(matches!(engine, Some(Engine::Native) | Some(Engine::Polling)));
}

// Core Watcher API: deleting a followed file surfaces a Removed event.
#[test]
fn watcher_reports_removed_event() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("removed.log");
    create_file(&path, b"x\n");

    let mut watcher = Watcher::new([&path], FollowOptions::default()).unwrap();

    let p = path.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        let _ = fs::remove_file(&p);
    });

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(6) {
            match watcher.recv() {
                Ok(Event::Removed(_)) => {
                    let _ = tx.send(true);
                    return;
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        let _ = tx.send(false);
    });

    assert!(
        rx.recv_timeout(Duration::from_secs(7)).unwrap_or(false),
        "expected a Removed event after deletion"
    );
}

// Core Watcher API: with a tight recheck cadence and no writes, recv() returns
// via the periodic tick that drives reconciliation rather than a backend event.
#[test]
fn watcher_tick_drives_reconcile_when_idle() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("idle.log");
    create_file(&path, b"y\n");

    let opts = FollowOptions {
        recheck_cadence: Duration::from_millis(50),
        poll_interval: Duration::from_millis(50),
        ..FollowOptions::default()
    };
    let mut watcher = Watcher::new([&path], opts).unwrap();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        // No writes occur, so the only way recv() returns is the idle tick.
        let _ = tx.send(watcher.recv().is_ok());
    });

    assert!(
        rx.recv_timeout(Duration::from_secs(3)).unwrap_or(false),
        "idle tick did not drive an event"
    );
}

// Multi-source following: next_chunk yields chunks tagged by the source that
// produced them.
#[test]
fn multi_source_next_chunk_tags_each_source() {
    let dir = tempfile::TempDir::new().unwrap();
    let a = dir.path().join("a.log");
    let b = dir.path().join("b.log");
    create_file(&a, b"a0\n");
    create_file(&b, b"b0\n");

    let mut f = Follower::new([&a, &b], FollowOptions::default()).unwrap();
    thread::sleep(Duration::from_millis(200));
    append_file(&a, b"aaa\n");
    append_file(&b, b"bbb\n");

    let mut seen_a = false;
    let mut seen_b = false;
    let start = Instant::now();
    while (!seen_a || !seen_b) && start.elapsed() < Duration::from_secs(5) {
        if let Ok(Some(chunk)) = f.next_chunk() {
            let text = String::from_utf8_lossy(&chunk.bytes).to_string();
            if text.contains("aaa") {
                seen_a = true;
            }
            if text.contains("bbb") {
                seen_b = true;
            }
        }
    }
    assert!(
        seen_a && seen_b,
        "expected data from both sources (a={seen_a}, b={seen_b})"
    );
}

// In-place truncation: after the file shrinks below the read offset, following
// seeks back to the start and delivers the rewritten content.
#[test]
fn truncation_re_reads_from_start() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("trunc.log");
    create_file(&path, b"first\n");

    let mut reader = fsmon::follow(&path).unwrap().into_reader();

    append_file(&path, b"aaaa\n");
    let mut buf = vec![0u8; 256];
    let mut got = Vec::new();
    let start = Instant::now();
    while !String::from_utf8_lossy(&got).contains("aaaa") && start.elapsed() < Duration::from_secs(5) {
        if let Ok(n) = reader.read(&mut buf) {
            got.extend_from_slice(&buf[..n]);
        }
    }

    // Truncate in place, then write fresh content shorter than the old offset.
    let f = fs::OpenOptions::new().write(true).open(&path).unwrap();
    f.set_len(0).unwrap();
    drop(f);
    append_file(&path, b"bbbb\n");

    let mut after = Vec::new();
    let start = Instant::now();
    while !String::from_utf8_lossy(&after).contains("bbbb") && start.elapsed() < Duration::from_secs(5) {
        if let Ok(n) = reader.read(&mut buf) {
            after.extend_from_slice(&buf[..n]);
        }
    }
    assert!(
        String::from_utf8_lossy(&after).contains("bbbb"),
        "post-truncation content not delivered: {:?}",
        String::from_utf8_lossy(&after)
    );
}

// Rotation: when the path's identity changes while the previously-open file
// still has unread bytes, those bytes are drained before following the new
// file (exercises the Draining state's leftover-data and reopen branches).
#[cfg(unix)]
#[test]
fn rotation_drains_old_fd_then_follows_new() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("rot.log");
    let rotated = dir.path().join("rot.log.1");
    create_file(&path, b"a\n");

    // Open the follower at EOF, then mutate before reading anything: append
    // unread bytes to the original file, rotate it away, and drop a new file in.
    let mut f = Follower::new([&path], FollowOptions::default()).unwrap();
    append_file(&path, b"leftover\n");
    fs::rename(&path, &rotated).unwrap();
    create_file(&path, b"brandnew\n");

    let acc = drain_until(&mut f, "brandnew", Duration::from_secs(8));
    let text = String::from_utf8_lossy(&acc);
    assert!(text.contains("leftover"), "old fd not drained: {text:?}");
    assert!(text.contains("brandnew"), "new file not followed: {text:?}");
}
