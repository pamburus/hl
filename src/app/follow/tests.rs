use super::*;

use rstest::rstest;

use crate::app::TimestampIndexLine;

const INTERVAL: Duration = Duration::from_millis(100);

fn ts(sec: i64) -> Timestamp {
    Timestamp { sec, nsec: 0 }
}

/// Returns block `n` with one record per item of `lines`, timestamped by its value.
fn block(n: usize, lines: &[i64]) -> (Vec<u8>, TimestampIndex) {
    let mut buf = Vec::new();
    let mut index = TimestampIndex::new(n);
    for &sec in lines {
        let start = buf.len();
        buf.extend(format!("  {sec}").as_bytes());
        index.lines.push(TimestampIndexLine {
            location: start..buf.len(),
            ts: ts(sec),
        });
        buf.push(b'\n');
    }
    (buf, index)
}

/// Returns block `n` with plain text and no records.
fn text(n: usize, text: &str) -> (Vec<u8>, TimestampIndex) {
    (text.as_bytes().to_vec(), TimestampIndex::new(n))
}

fn window(mem_limit: usize) -> MergeWindow<'static> {
    MergeWindow::new(b"\n", mem_limit, INTERVAL)
}

fn insert(window: &mut MergeWindow, (buf, index): (Vec<u8>, TimestampIndex), now: Instant) -> Option<String> {
    window
        .insert(0, buf, index, now)
        .map(|(buf, location)| String::from_utf8(buf[location].to_vec()).unwrap())
}

fn drain(window: &mut MergeWindow, now: Instant, all: bool) -> Vec<String> {
    std::iter::from_fn(|| window.pop_due(now, all))
        .map(|(_, entry)| String::from_utf8(entry.data().to_vec()).unwrap())
        .collect()
}

// ---

#[test]
fn test_merge_window_gives_out_entries_by_time() {
    let mut window = window(usize::MAX);
    let now = Instant::now();
    insert(&mut window, block(0, &[3, 1]), now);
    insert(&mut window, block(1, &[2]), now);

    assert_eq!(drain(&mut window, now, true), ["  1", "  2", "  3"]);
}

#[rstest]
#[case::received(Duration::ZERO, &[])]
#[case::almost_due(INTERVAL - Duration::from_millis(1), &[])]
#[case::due(INTERVAL, &["  1"])]
fn test_merge_window_pop_due_after_sync_interval(#[case] elapsed: Duration, #[case] expected: &[&str]) {
    let mut window = window(usize::MAX);
    let now = Instant::now();
    insert(&mut window, block(0, &[1]), now);

    assert_eq!(drain(&mut window, now + elapsed, false), expected);
}

#[test]
fn test_merge_window_pop_due_when_memory_limit_is_reached() {
    // Each entry takes 3 bytes, so two entries fill the window and one is given out early.
    let mut window = window(6);
    let now = Instant::now();
    insert(&mut window, block(0, &[1, 2]), now);

    assert_eq!(drain(&mut window, now, false), ["  1"]);
    assert_eq!(drain(&mut window, now, true), ["  2"]);
}

#[test]
fn test_merge_window_wait() {
    let mut window = window(usize::MAX);
    let now = Instant::now();
    assert_eq!(window.wait(now), Duration::MAX);

    insert(&mut window, block(0, &[1]), now);
    assert_eq!(window.wait(now + Duration::from_millis(30)), Duration::from_millis(70));
    assert_eq!(window.wait(now + INTERVAL * 2), Duration::ZERO);
}

#[test]
fn test_merge_window_sorts_text_only_after_a_record() {
    let mut window = window(usize::MAX);
    let now = Instant::now();

    assert_eq!(
        insert(&mut window, text(0, "before\n"), now).as_deref(),
        Some("before\n")
    );
    assert_eq!(insert(&mut window, block(1, &[1]), now), None);
    assert_eq!(insert(&mut window, text(2, "after\n"), now), None);
    assert_eq!(drain(&mut window, now, true), ["  1", "after"]);
}

#[test]
fn test_merge_window_skips_empty_text() {
    let mut window = window(usize::MAX);
    let now = Instant::now();

    assert_eq!(insert(&mut window, text(0, "\n"), now), None);
    assert_eq!(insert(&mut window, block(1, &[1]), now), None);
    assert_eq!(insert(&mut window, text(2, "\n"), now), None);
    assert_eq!(drain(&mut window, now, true), ["  1"]);
}

#[test]
fn test_merge_writer_marks_entries_out_of_time_order() {
    let si = SyncIndicator {
        width: 2,
        synced: "S ".into(),
        failed: "F ".into(),
        placeholder: "  ".into(),
    };
    let mut writer = MergeWriter::new(&si, b"\n");
    let mut output = Vec::new();
    for (sec, entry) in [(2, "  a"), (1, "  b"), (1, "  c"), (3, "  d")] {
        writer.write(&mut output, ts(sec), entry.as_bytes()).unwrap();
    }

    assert_eq!(String::from_utf8(output).unwrap(), "S a\nF b\nS c\nS d\n");
}
