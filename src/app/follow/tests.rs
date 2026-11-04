use super::*;

use rstest::rstest;

use crate::app::{SyncIndicator, TimestampIndexLine};

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

/// Returns block `n` with a part of a log entry too large to be read at once.
fn part(n: usize, text: &str, part: PartialPlacement) -> (Vec<u8>, TimestampIndex) {
    let index = TimestampIndex {
        part: Some(part),
        ..TimestampIndex::new(n)
    };
    (text.as_bytes().to_vec(), index)
}

fn entry(text: &str, part: Option<PartialPlacement>) -> Entry {
    let buf = Rc::new(text.as_bytes().to_vec());
    Entry::new(&buf, 0..buf.len(), Instant::now(), part)
}

fn window(mem_limit: usize) -> MergeWindow<'static> {
    MergeWindow::new(b"\n", mem_limit, INTERVAL)
}

fn insert(window: &mut MergeWindow, (buf, index): (Vec<u8>, TimestampIndex), now: Instant) {
    window.insert(0, buf, index, now)
}

/// Returns what is due, with the end of a split entry shown as `|`, and a part of a split entry
/// that starts a new line, as the entry was ended before the part came, marked by `+`.
fn drain(window: &mut MergeWindow, now: Instant, all: bool) -> Vec<String> {
    std::iter::from_fn(|| window.pop_due(now, all))
        .map(|due| match due {
            Due::Entry(_, entry) => String::from_utf8(entry.data().to_vec()).unwrap(),
            Due::Resumed(_, entry) => format!("+{}", String::from_utf8(entry.data().to_vec()).unwrap()),
            Due::SplitEnd => "|".into(),
        })
        .collect()
}

fn badges(input: Option<Vec<String>>) -> FollowBadges {
    FollowBadges {
        si: SyncIndicator {
            width: 2,
            synced: "S ".into(),
            failed: "F ".into(),
            placeholder: "  ".into(),
        },
        input,
    }
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
    // Text with no record before it has nothing to be sorted by, so it is due right away.
    let mut window = window(usize::MAX);
    let now = Instant::now();
    insert(&mut window, text(0, "before\n"), now);
    insert(&mut window, block(1, &[1]), now);
    insert(&mut window, text(2, "after\n"), now);

    assert_eq!(drain(&mut window, now, false), ["before"]);
    assert_eq!(drain(&mut window, now, true), ["  1", "after"]);
}

#[rstest]
#[case::after_first_record(&[], "  1\ntrace\n", &["  1", "trace"])]
#[case::between_records(&[5], "  9\ntrace\n  2\n", &["  2", "  5", "  9", "trace"])]
#[case::out_of_order(&[5], "  3\ntrace\n", &["  3", "trace", "  5"])]
fn test_merge_window_sorts_text_with_record_before_it(
    #[case] before: &[i64],
    #[case] last: &str,
    #[case] expected: &[&str],
) {
    let mut window = window(usize::MAX);
    let now = Instant::now();
    insert(&mut window, block(0, before), now);

    // Records in `last` are the lines starting with spaces, timestamped by their value.
    let mut index = TimestampIndex::new(1);
    let mut start = 0;
    for line in last.split_inclusive('\n') {
        let text = line.trim_end_matches('\n');
        if let Some(sec) = text.strip_prefix("  ") {
            index.lines.push(TimestampIndexLine {
                location: start..start + text.len(),
                ts: ts(sec.parse().unwrap()),
            });
        }
        start += line.len();
    }
    insert(&mut window, (last.as_bytes().to_vec(), index), now);
    assert_eq!(drain(&mut window, now, true), expected);
}

#[test]
fn test_merge_window_skips_empty_text() {
    let mut window = window(usize::MAX);
    let now = Instant::now();

    insert(&mut window, text(0, "\n"), now);
    insert(&mut window, block(1, &[1]), now);
    insert(&mut window, text(2, "\n"), now);
    assert_eq!(drain(&mut window, now, true), ["  1"]);
}

#[test]
fn test_merge_window_takes_blocks_in_order() {
    let mut window = window(usize::MAX);
    let now = Instant::now();
    insert(&mut window, block(2, &[3]), now);
    insert(&mut window, text(1, "trace\n"), now);
    assert_eq!(drain(&mut window, now, true), Vec::<String>::new());

    insert(&mut window, block(0, &[1]), now);
    assert_eq!(drain(&mut window, now, true), ["  1", "trace", "  3"]);
}

#[test]
fn test_merge_window_holds_entries_while_split_entry_is_open() {
    let mut window = window(usize::MAX);
    let now = Instant::now();
    insert(&mut window, block(0, &[1]), now);
    insert(&mut window, part(1, "  xx", PartialPlacement::First), now);
    let (buf, index) = block(0, &[2]);
    window.insert(1, buf, index, now);

    // Until the rest of the split entry comes, nothing else is given out, even if due.
    let later = now + INTERVAL;
    assert_eq!(drain(&mut window, later, false), ["  1", "  xx"]);
    let soon = later + Duration::from_millis(30);
    assert_eq!(window.wait(soon), Duration::from_millis(70));
    assert_eq!(drain(&mut window, soon, false), Vec::<String>::new());

    insert(&mut window, part(2, "yy", PartialPlacement::Next), now);
    insert(&mut window, part(3, "zz", PartialPlacement::Last), now);
    assert_eq!(drain(&mut window, soon, false), ["yy", "zz", "  2"]);
}

#[rstest]
#[case::awaited(false, INTERVAL - Duration::from_millis(1), &[])]
#[case::by_next_block(true, Duration::ZERO, &["|"])]
#[case::when_late(false, INTERVAL, &["|"])]
fn test_merge_window_ends_split_entry(#[case] next_block: bool, #[case] elapsed: Duration, #[case] expected: &[&str]) {
    let mut window = window(usize::MAX);
    let now = Instant::now();
    insert(&mut window, part(0, "  xx", PartialPlacement::First), now);
    assert_eq!(drain(&mut window, now, false), ["  xx"]);
    if next_block {
        insert(&mut window, block(1, &[2]), now);
    }

    assert_eq!(drain(&mut window, now + elapsed, false), expected);
}

#[test]
fn test_merge_window_ends_split_entry_at_twice_memory_limit() {
    // Each record takes 3 bytes, so two records of the other input fill the window,
    // and four of them fill it twice.
    let mut window = window(6);
    let now = Instant::now();
    insert(&mut window, part(0, "  xx", PartialPlacement::First), now);
    assert_eq!(drain(&mut window, now, false), ["  xx"]);
    let (buf, index) = block(0, &[1, 2]);
    window.insert(1, buf, index, now);
    assert_eq!(drain(&mut window, now, false), Vec::<String>::new());

    // Then the window is written down to below its memory limit.
    let (buf, index) = block(1, &[3, 4]);
    window.insert(1, buf, index, now);
    assert_eq!(drain(&mut window, now, false), ["|", "  1", "  2", "  3"]);

    // The rest is given out as it comes.
    insert(&mut window, part(1, "yy", PartialPlacement::Last), now);
    assert_eq!(drain(&mut window, now, false), ["+yy"]);
    assert_eq!(drain(&mut window, now, true), ["  4"]);
    assert!(window.late.is_empty());
}

#[test]
fn test_merge_window_gives_out_rest_of_split_entry_ended_early() {
    let mut window = window(usize::MAX);
    let now = Instant::now();
    insert(&mut window, part(0, "  xx", PartialPlacement::First), now);
    assert_eq!(drain(&mut window, now, false), ["  xx"]);
    let later = now + INTERVAL;
    assert_eq!(drain(&mut window, later, false), ["|"]);

    // The rest is given out as it comes, and held together again until it ends.
    insert(&mut window, part(1, "yy", PartialPlacement::Next), later);
    assert_eq!(drain(&mut window, later, false), ["+yy"]);
    let (buf, index) = block(0, &[1]);
    window.insert(1, buf, index, now);
    assert_eq!(drain(&mut window, later, false), Vec::<String>::new());

    insert(&mut window, part(2, "zz", PartialPlacement::Last), later);
    assert_eq!(drain(&mut window, later, false), ["zz", "  1"]);
    assert!(window.late.is_empty());
}

#[test]
fn test_merge_window_forgets_split_entry_when_input_moves_on() {
    let mut window = window(usize::MAX);
    let now = Instant::now();
    insert(&mut window, part(0, "  xx", PartialPlacement::First), now);
    assert_eq!(drain(&mut window, now, false), ["  xx"]);
    let later = now + INTERVAL;
    assert_eq!(drain(&mut window, later, false), ["|"]);

    insert(&mut window, block(1, &[1]), later);
    assert_eq!(drain(&mut window, later, true), ["  1"]);
    assert!(window.late.is_empty());
}

#[test]
fn test_merge_writer_marks_entries_out_of_time_order() {
    let badges = badges(None);
    let mut writer = MergeWriter::new(&badges, b"\n");
    let mut output = Vec::new();
    for (sec, text) in [(2, "  a"), (1, "  b"), (1, "  c"), (3, "  d")] {
        writer.write(&mut output, Some(ts(sec)), &entry(text, None)).unwrap();
    }

    assert_eq!(String::from_utf8(output).unwrap(), "S a\nF b\nS c\nS d\n");
}

#[test]
fn test_merge_writer_writes_split_entries() {
    let badges = badges(None);
    let mut writer = MergeWriter::new(&badges, b"\n");
    let mut output = Vec::new();
    writer.write(&mut output, None, &entry("  text", None)).unwrap();
    writer
        .write(&mut output, Some(ts(1)), &entry("  xx", Some(PartialPlacement::First)))
        .unwrap();
    writer
        .write(&mut output, Some(ts(1)), &entry("yy", Some(PartialPlacement::Next)))
        .unwrap();
    writer
        .write(&mut output, Some(ts(1)), &entry("zz", Some(PartialPlacement::Last)))
        .unwrap();
    writer
        .write(&mut output, None, &entry("  aa", Some(PartialPlacement::First)))
        .unwrap();
    writer.end_split(&mut output).unwrap();

    assert_eq!(String::from_utf8(output).unwrap(), "  text\nS xxyyzz\n  aa\n");
}

#[test]
fn test_merge_writer_resumes_split_entry_with_input_badge() {
    let badges = badges(Some(vec!["  #0 ".into(), "  #1 ".into()]));
    let mut writer = MergeWriter::new(&badges, b"\n");
    let mut output = Vec::new();
    writer
        .write(
            &mut output,
            Some(ts(1)),
            &entry("  #1 xx", Some(PartialPlacement::First)),
        )
        .unwrap();
    writer.end_split(&mut output).unwrap();
    writer
        .resume(&mut output, 1, &entry("yy", Some(PartialPlacement::Next)))
        .unwrap();
    writer
        .write(&mut output, None, &entry("zz", Some(PartialPlacement::Last)))
        .unwrap();

    assert_eq!(String::from_utf8(output).unwrap(), "S #1 xx\n  #1 yyzz\n");
}
