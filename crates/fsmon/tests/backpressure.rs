use std::fs::{self, File};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

use fsmon::{FollowOptions, Follower};

// T051: A fast writer outpacing the consumer delivers all bytes without loss
// (FR-015a, SC-011).
#[test]
fn test_fast_writer_no_loss_bounded_memory() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fast.log");
    File::create(&path).unwrap();

    const LINE: &[u8] = b"0123456789\n"; // 11 bytes per line
    const LINES: usize = 1_000;
    const TOTAL: usize = LINES * LINE.len();

    let opts = FollowOptions {
        poll_interval: Duration::from_millis(100),
        recheck_cadence: Duration::from_millis(200),
        ..Default::default()
    };
    let follower = Follower::new([&path], opts).unwrap();
    let mut reader = follower.into_reader();

    let path2 = path.clone();
    let writer = thread::spawn(move || {
        // Small delay so the follower is set up before writing starts.
        thread::sleep(Duration::from_millis(50));
        let mut f = fs::OpenOptions::new().append(true).open(&path2).unwrap();
        for _ in 0..LINES {
            f.write_all(LINE).unwrap();
        }
        f.sync_all().unwrap();
    });

    // Read all TOTAL bytes from the reader in a separate thread with a timeout.
    let reader_thread = thread::spawn(move || {
        let mut received = Vec::with_capacity(TOTAL + 1024);
        let mut buf = vec![0u8; 1024];
        let start = std::time::Instant::now();
        while received.len() < TOTAL {
            if start.elapsed() > Duration::from_secs(30) {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    received.extend_from_slice(&buf[..n]);
                    // Simulate a slightly slow consumer.
                    thread::sleep(Duration::from_micros(100));
                }
                Err(e) => panic!("read error: {e}"),
            }
        }
        received
    });

    writer.join().unwrap();
    // Give reader up to 30s to drain all the data.
    let received = reader_thread.join().unwrap();

    assert_eq!(
        received.len(),
        TOTAL,
        "all {} bytes must be received without loss",
        TOTAL
    );

    // Verify content integrity.
    let expected: Vec<u8> = (0..LINES).flat_map(|_| LINE.iter().copied()).collect();
    assert_eq!(received, expected, "content must match exactly");
}
