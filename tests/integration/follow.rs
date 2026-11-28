//! Following files with `--follow`, as specified in `specs/011-file-following/spec.md`.

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use rstest::rstest;
use tempfile::TempDir;

use crate::support::{
    hl::Hl,
    log::{append, record},
};

/// How long a test waits for what it expects to be shown.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Creates a new directory with an empty file for each of `names`.
///
/// The directory is returned with the files, so it is removed only once the test ends.
fn files<const N: usize>(names: [&str; N]) -> (TempDir, [PathBuf; N]) {
    let dir = TempDir::new().unwrap();
    let files = names.map(|name| {
        let file = dir.path().join(format!("{name}.log"));
        append(&file, "");
        file
    });
    (dir, files)
}

/// Returns the arguments to follow `files` with the options in `args`.
fn args(files: &[PathBuf], args: &[&str]) -> Vec<OsString> {
    let options = ["--follow"].iter().chain(args).map(OsString::from);
    options.chain(files.iter().map(OsString::from)).collect()
}

/// Appends a record to each of `files` and waits until it is shown, so that what is appended next is read in a pass of its own.
///
/// Its timestamp is earlier than those of the tests, so it does not change the order of the records they append.
fn wait_ready(hl: &mut Hl, files: &[PathBuf]) {
    for (i, file) in files.iter().enumerate() {
        let msg = format!("ready-{i}");
        append(file, record("2026-07-06T09:00:00Z", &msg));
        assert!(hl.wait_entry(&msg, TIMEOUT).is_some(), "{msg} was not shown");
    }
}

/// Starts following `files` with the options in `args`, once each of them is read.
fn follow(files: &[PathBuf], options: &[&str]) -> Hl {
    let mut hl = Hl::spawn(args(files, options));
    wait_ready(&mut hl, files);
    hl
}

/// Appends `parts` to `file` one by one, with a pause after each but the last one.
fn append_parts(file: &Path, parts: &[impl AsRef<[u8]>], pause: Duration) {
    for (i, part) in parts.iter().enumerate() {
        if i != 0 {
            thread::sleep(pause);
        }
        append(file, part);
    }
}

/// Moves the file at `path` away, as a log rotation does, so that the next write creates a new one.
fn rotate(path: &Path) {
    let mut rotated = path.as_os_str().to_owned();
    rotated.push(".1");
    fs::rename(path, rotated).unwrap();
}

/// Truncates the file at `path` to nothing, as `copytruncate` log rotation does.
fn truncate(path: &Path) {
    fs::File::create(path).unwrap();
}

/// Removes the file at `path`, so that the next write creates a new one.
fn remove(path: &Path) {
    fs::remove_file(path).unwrap();
}

/// Makes a FIFO at `path`.
#[cfg(unix)]
fn mkfifo(path: &Path) {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let c = CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: `c` is a valid C string that outlives the call.
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0, "mkfifo failed");
}

/// Writes `data` to the FIFO at `path` from a thread of its own once the FIFO has a reader, which it waits for until
/// [`TIMEOUT`] passes, so that a test whose reader never comes does not leave it blocked.
#[cfg(unix)]
fn write_fifo(path: &Path, data: String) -> thread::JoinHandle<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt, time::Instant};
    let path = path.to_owned();
    thread::spawn(move || {
        let deadline = Instant::now() + TIMEOUT;
        // A FIFO opened for writing without blocking fails until it has a reader.
        let mut file = loop {
            match fs::OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&path)
            {
                Ok(file) => break file,
                Err(err) if err.raw_os_error() == Some(libc::ENXIO) && Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("cannot open {} for writing: {err}", path.display()),
            }
        };
        // The data is far smaller than the buffer of the FIFO, so it is written at once.
        file.write_all(data.as_bytes()).unwrap()
    })
}

/// Makes an encoder of a compression format that writes to a file.
type Encoder = fn(fs::File) -> Box<dyn std::io::Write>;

fn gzip(file: fs::File) -> Box<dyn std::io::Write> {
    Box::new(flate2::write::GzEncoder::new(file, flate2::Compression::default()))
}

fn bzip2(file: fs::File) -> Box<dyn std::io::Write> {
    Box::new(bzip2::write::BzEncoder::new(file, bzip2::Compression::default()))
}

/// Writes `data` to a new file at `path`, compressed with `encoder`.
fn write_compressed(path: &Path, encoder: Encoder, data: &str) {
    let mut encoder = encoder(fs::File::create(path).unwrap());
    encoder.write_all(data.as_bytes()).unwrap();
    // The encoder finishes its data when it is dropped.
    drop(encoder);
}

/// FR-003, FR-008: a file is followed from its last entries, as many as `--tail` asks for, and then from what is appended to it,
/// whichever input format, and so delimiter of entries, is used.
#[rstest]
fn test_shows_last_entries_then_appended_ones(#[values("auto", "json")] format: &str) {
    let (_dir, files) = files(["a"]);
    for i in 0..3 {
        append(&files[0], record("2026-07-06T10:00:00Z", &format!("old-{i}")));
    }
    let mut hl = Hl::spawn(args(&files, &["--tail", "1", "--input-format", format]));

    let entries = hl.next_entries(1, TIMEOUT);
    assert!(
        entries.len() == 1 && entries[0].contains("old-2"),
        "the last entry was not shown first: {entries:?}"
    );
    append(&files[0], record("2026-07-06T10:00:01Z", "new"));
    let entries = hl.next_entries(1, TIMEOUT);
    assert!(
        entries.len() == 1 && entries[0].contains("new"),
        "the appended entry was not shown next: {entries:?}"
    );
}

/// FR-004, FR-005, FR-007, FR-024: once the file at a followed path is rotated, truncated or removed,
/// the entries written to the file that takes its place are shown from its start.
#[rstest]
#[case::rotated(rotate)]
#[case::truncated(truncate)]
#[case::removed(remove)]
fn test_follows_file_that_takes_place_of_another(#[case] change: fn(&Path)) {
    let (_dir, files) = files(["a"]);
    let file = &files[0];
    let mut hl = follow(&files, &[]);
    // The replaced file is longer than what the new one gets, so a truncation is seen as one.
    for i in 0..3 {
        let msg = format!("old-{i}");
        append(file, record("2026-07-06T10:00:00Z", &msg));
        assert!(hl.wait_entry(&msg, TIMEOUT).is_some(), "{msg} was not shown");
    }

    change(file);
    // The new file gets two writes, so that the first one is shown even if it is written before the new file is found.
    append(file, record("2026-07-06T10:00:01Z", "new-0"));
    thread::sleep(Duration::from_millis(200));
    append(file, record("2026-07-06T10:00:02Z", "new-1"));

    let entries = hl.next_entries(2, TIMEOUT);
    assert_eq!(
        entries.len(),
        2,
        "the entries of the new file were not shown: {entries:?}"
    );
    for (entry, msg) in entries.iter().zip(["new-0", "new-1"]) {
        assert!(entry.contains(msg), "expected {msg} in this order: {entries:?}");
    }
}

/// Edge case "Symlinked target": once a followed link is pointed at another file, as `tail -F` does, the entries of
/// that file are shown from its start, and those written to the file it pointed at before are not.
#[cfg(unix)]
#[test]
fn test_follows_link_pointed_at_another_file() {
    let (dir, targets) = files(["a", "b"]);
    let link = dir.path().join("current.log");
    std::os::unix::fs::symlink(&targets[0], &link).unwrap();
    let mut hl = follow(std::slice::from_ref(&link), &[]);

    // The link is replaced at once, as `ln -sfn` does.
    let new_link = dir.path().join("current.log.new");
    std::os::unix::fs::symlink(&targets[1], &new_link).unwrap();
    fs::rename(&new_link, &link).unwrap();
    // The new file gets two writes, so that the first one is shown even if it is written before the link is resolved again.
    append(&targets[1], record("2026-07-06T10:00:01Z", "new-0"));
    thread::sleep(Duration::from_millis(200));
    append(&targets[1], record("2026-07-06T10:00:02Z", "new-1"));

    let entries = hl.next_entries(2, TIMEOUT);
    assert!(
        entries.len() == 2 && entries[0].contains("new-0") && entries[1].contains("new-1"),
        "the entries of the file the link points at now were not shown: {entries:?}"
    );
    append(&targets[0], record("2026-07-06T10:00:03Z", "old"));
    let again = hl.next_entries(1, Duration::from_secs(1));
    assert!(
        again.is_empty(),
        "the file the link pointed at before is still followed: {again:?}"
    );
}

/// FR-002: a file that is missing as following starts is waited for, as with `tail -F`, and once it appears, its entries
/// are shown from its start.
#[test]
fn test_waits_for_missing_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.log");
    let mut hl = Hl::spawn(args(std::slice::from_ref(&path), &[]));

    thread::sleep(Duration::from_millis(300));
    append(&path, record("2026-07-06T10:00:00Z", "first"));
    let entries = hl.next_entries(1, TIMEOUT);
    assert!(
        entries.len() == 1 && entries[0].contains("first"),
        "the entry of the file that appeared was not shown: {entries:?}"
    );
}

/// FR-010: a missing file given by a bare name, which is in the current directory, is waited for with native
/// notifications, as one given by any other path on a local file system is.
#[test]
fn test_waits_for_missing_file_given_by_bare_name_natively() {
    let (dir, files) = files(["b"]);
    let mut hl = Hl::spawn_in_with_debug_log(dir.path(), ["--follow", "a.log", "b.log"], "debug");
    // Once the file that is there is followed, the missing one is waited for, as both are set up as following starts.
    wait_ready(&mut hl, &files);
    append(&dir.path().join("a.log"), record("2026-07-06T10:00:00Z", "first"));
    assert!(hl.wait_entry("first", TIMEOUT).is_some(), "first was not shown");

    let log = hl.errors();
    assert!(
        log.lines().any(|line| line.contains("fsmon: waiting for a.log")),
        "the missing file was not waited for: {log}"
    );
    assert!(
        !log.lines()
            .any(|line| line.contains("polling") && line.contains("a.log")),
        "the missing file was polled for: {log}"
    );
}

/// FR-002: a file that cannot be read as following starts is waited for, as with `tail -F`, and once it can be read,
/// its entries are shown from its start.
#[cfg(unix)]
#[test]
fn test_waits_for_unreadable_file() {
    use std::os::unix::fs::PermissionsExt;
    // SAFETY: the call takes no arguments.
    if unsafe { libc::geteuid() } == 0 {
        // Permissions do not keep the superuser from reading a file.
        return;
    }
    let (_dir, files) = files(["a"]);
    append(&files[0], record("2026-07-06T10:00:00Z", "first"));
    fs::set_permissions(&files[0], fs::Permissions::from_mode(0o000)).unwrap();
    let mut hl = Hl::spawn(args(&files, &[]));

    thread::sleep(Duration::from_millis(300));
    fs::set_permissions(&files[0], fs::Permissions::from_mode(0o644)).unwrap();
    let entries = hl.next_entries(1, TIMEOUT);
    assert!(
        entries.len() == 1 && entries[0].contains("first"),
        "the entry of the file that became readable was not shown: {entries:?}"
    );
}

/// SC-004: following idle files does not busy-wait. The limit is far above what idle following uses, so that the coarse
/// clock of the CPU time does not make the test flaky, and far below what a busy wait uses, which is a whole core or more.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn test_idle_following_uses_little_cpu() {
    let names: [String; 10] = std::array::from_fn(|i| i.to_string());
    let (_dir, files) = files(names.each_ref().map(String::as_str));
    let hl = follow(&files, &[]);

    let idle = Duration::from_secs(1);
    let before = hl.cpu_time();
    thread::sleep(idle);
    let used = hl.cpu_time().saturating_sub(before);
    assert!(
        used < idle / 10,
        "{used:?} of CPU time was used in {idle:?} of idle following"
    );
}

/// FR-016b: what `tail -F` reports on its standard error goes to the debug log, as a warning for a path that cannot be
/// opened, and as information for a file that is replaced or removed, and for a file that is followed from its start.
#[test]
fn test_reports_changes_in_debug_log() {
    let (dir, files) = files(["b"]);
    let missing = dir.path().join("a.log");
    let file = &files[0];
    let mut hl = Hl::spawn_with_debug_log(args(&[missing, file.clone()], &[]), "info");

    append(file, record("2026-07-06T10:00:00Z", "first"));
    assert!(hl.wait_entry("first", TIMEOUT).is_some(), "first was not shown");
    remove(file);
    append(file, record("2026-07-06T10:00:01Z", "second"));
    assert!(hl.wait_entry("second", TIMEOUT).is_some(), "second was not shown");

    let log = hl.errors();
    let logged = |level: &str, text: &str, name: &str| {
        log.lines()
            .any(|line| line.contains(level) && line.contains(text) && line.contains(name))
    };
    assert!(
        logged("WARN", "waiting for", "a.log"),
        "the missing file was not reported: {log}"
    );
    assert!(
        logged("INFO", "was replaced or removed", "b.log"),
        "the removal was not reported: {log}"
    );
    assert!(
        logged("INFO", "following", "b.log"),
        "following the new file was not reported: {log}"
    );
}

/// FR-016c: an error reading a followed input ends following, with the error reported, as `tail -F` exits on one, even
/// while another input could still be followed.
#[test]
fn test_exits_on_read_error() {
    let (dir, files) = files(["b"]);
    let corrupt = dir.path().join("a.log.gz");
    // A gzip header followed by a block of a type that does not exist.
    fs::write(&corrupt, [0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]).unwrap();
    let mut hl = Hl::spawn(args(&[corrupt, files[0].clone()], &[]));
    let (_, status) = hl.wait_output(TIMEOUT);
    assert!(!status.success(), "hl did not fail: {status}");
    let errors = hl.errors();
    assert!(errors.contains("a.log.gz"), "the error was not reported: {errors}");
}

/// FR-016c, FR-036: an error reading another input ends following of the standard input at once, even while it waits
/// for data with its writer still open.
#[test]
fn test_read_error_ends_following_of_stdin() {
    use std::io::Write;

    let dir = TempDir::new().unwrap();
    let corrupt = dir.path().join("a.log.gz");
    let (mut hl, mut stdin) = Hl::spawn_with_stdin(args(&[PathBuf::from("-"), corrupt.clone()], &[]));
    // Once shown, the standard input is read again, which waits for the next data.
    stdin
        .write_all(record("2026-07-06T09:00:00Z", "ready").as_bytes())
        .unwrap();
    assert!(hl.wait_entry("ready", TIMEOUT).is_some(), "the record was not shown");

    // A gzip header followed by a block of a type that does not exist, in a file that appears once it is waited for.
    fs::write(&corrupt, [0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]).unwrap();
    let (_, status) = hl.wait_output(TIMEOUT);
    assert!(!status.success(), "hl did not fail: {status}");
    drop(stdin);
}

/// FR-026: a directory given to follow is waited for, as with `tail -F`, while the other inputs are followed, and the
/// file that takes its place is followed from its start.
#[test]
fn test_waits_for_directory_to_be_replaced_by_file() {
    let (dir, files) = files(["b"]);
    let path = dir.path().join("a.log");
    fs::create_dir(&path).unwrap();
    let mut hl = Hl::spawn(args(&[path.clone(), files[0].clone()], &[]));
    wait_ready(&mut hl, &files);

    fs::remove_dir(&path).unwrap();
    append(&path, record("2026-07-06T10:00:00Z", "first"));
    assert!(
        hl.wait_entry("first", TIMEOUT).is_some(),
        "the new file was not followed"
    );
}

/// FR-026: a character device given to follow, such as `/dev/null`, is followed as with `tail -F`, along with the
/// other inputs.
#[cfg(unix)]
#[test]
fn test_follows_character_device() {
    let (_dir, files) = files(["a"]);
    let mut hl = Hl::spawn(args(&[PathBuf::from("/dev/null"), files[0].clone()], &[]));
    wait_ready(&mut hl, &files);
    append(&files[0], record("2026-07-06T10:00:00Z", "first"));
    assert!(hl.wait_entry("first", TIMEOUT).is_some(), "the file was not followed");
}

/// FR-008: the last entries of a file that cannot be seeked from its end, as in procfs, or that is shorter than its
/// size, as in sysfs, are shown, as with `tail -F`, as they are found by reading it from its start.
#[cfg(target_os = "linux")]
#[rstest]
#[case::procfs("/proc/version")]
#[case::sysfs("/sys/kernel/uevent_seqnum")]
fn test_follows_file_of_kernel_file_system(#[case] path: &str) {
    let mut hl = Hl::spawn(args(&[PathBuf::from(path)], &[]));
    assert!(!hl.next_entries(1, TIMEOUT).is_empty(), "{path} was not shown");
}

/// FR-038: a compressed file is followed decoded, from its start, as it is shown when it is not followed, whatever the
/// number of last entries asked for, and is shown once, also in a format whose data ends before the end of the file.
///
/// The unterminated entry at the end of the file is shown too in a format whose data cannot be continued once it ends.
/// A gzip file can have more data appended to it, so its unterminated entry waits for it, and is not checked.
#[rstest]
#[case::gzip("a.log.gz", gzip, "\n")]
#[case::bzip2("a.log.bz2", bzip2, "")]
fn test_follows_compressed_file(#[case] name: &str, #[case] encoder: Encoder, #[case] end: &str) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join(name);
    let data = record("2026-07-06T10:00:00Z", "first") + record("2026-07-06T10:00:00Z", "second").trim_end() + end;
    write_compressed(&file, encoder, &data);

    // Only the last entry is asked for, but the whole file is shown.
    let mut hl = Hl::spawn(args(std::slice::from_ref(&file), &["--tail", "1"]));
    let entries = hl.next_entries(2, TIMEOUT);
    assert!(
        entries.len() == 2 && entries[0].contains("first") && entries[1].contains("second"),
        "the decoded entries were not shown: {entries:?}"
    );
    let again = hl.next_entries(1, Duration::from_secs(1));
    assert!(again.is_empty(), "the decoded entries were shown again: {again:?}");
}

/// FR-005, FR-038: a compressed file that takes the place of a followed file that is not compressed is shown decoded.
#[rstest]
fn test_follows_compressed_file_that_takes_place_of_another(#[values(gzip, bzip2)] encoder: Encoder) {
    let (dir, files) = files(["a"]);
    let mut hl = follow(&files, &[]);

    let new = dir.path().join("new.log");
    let data = record("2026-07-06T10:00:00Z", "first") + &record("2026-07-06T10:00:00Z", "second");
    write_compressed(&new, encoder, &data);
    fs::rename(&new, &files[0]).unwrap();

    let entries = hl.next_entries(2, TIMEOUT);
    assert!(
        entries.len() == 2 && entries[0].contains("first") && entries[1].contains("second"),
        "the decoded entries of the new file were not shown: {entries:?}"
    );
}

/// FR-026: the entries of each writer of a FIFO are shown, the unterminated text at the end of what a writer writes
/// once it closes the FIFO, and then those of the next writer.
#[cfg(unix)]
#[test]
fn test_follows_fifo_across_writers() {
    let dir = TempDir::new().unwrap();
    let fifo = dir.path().join("a.fifo");
    mkfifo(&fifo);

    let mut hl = Hl::spawn(args(std::slice::from_ref(&fifo), &[]));
    // Each writer is ready only once hl opens the FIFO.
    let write = |data: String| write_fifo(&fifo, data);

    let first = write(format!("{}unterminated", record("2026-07-06T10:00:00Z", "first")));
    let entries = hl.next_entries(2, TIMEOUT);
    // The writer is joined only once its entries are shown, as it waits for hl to open the FIFO.
    assert_eq!(
        entries.len(),
        2,
        "the entries of the first writer were not shown: {entries:?}"
    );
    assert!(
        entries[0].contains("first") && entries[1].contains("unterminated"),
        "{entries:?}"
    );
    first.join().unwrap();

    let second = write(record("2026-07-06T10:00:01Z", "second"));
    assert!(
        hl.wait_entry("second", TIMEOUT).is_some(),
        "the entry of the second writer was not shown"
    );
    second.join().unwrap();
}

/// FR-001, FR-026: a FIFO with no writer yet does not hold back the other inputs, and what its first writer writes is
/// shown once it comes, as with `tail -F`.
#[cfg(unix)]
#[test]
fn test_fifo_with_no_writer_does_not_hold_back_other_inputs() {
    let (dir, files) = files(["b"]);
    let fifo = dir.path().join("a.fifo");
    mkfifo(&fifo);
    let mut hl = Hl::spawn(args(&[fifo.clone(), files[0].clone()], &[]));
    // The regular file is shown while the FIFO still has no writer.
    wait_ready(&mut hl, &files);

    let writer = write_fifo(&fifo, record("2026-07-06T10:00:00Z", "from-fifo"));
    assert!(
        hl.wait_entry("from-fifo", TIMEOUT).is_some(),
        "the entry of the FIFO was not shown"
    );
    writer.join().unwrap();
}

/// FR-027: records with equal timestamps are all shown, also when the same input gets them at different times, in the
/// order of their inputs and then of their position.
#[test]
fn test_shows_records_with_equal_timestamps() {
    let (_dir, files) = files(["a", "b"]);
    // The sync interval is long, so all the records are held together.
    let mut hl = follow(&files, &["--sync-interval-ms", "1000"]);

    let ts = "2026-07-06T10:00:00Z";
    for round in 0..2 {
        for (file, name) in files.iter().zip(["a", "b"]) {
            append(file, record(ts, &format!("{name}-{round}")));
        }
        thread::sleep(Duration::from_millis(200));
    }

    let entries = hl.next_entries(4, TIMEOUT);
    let expected = ["a-0", "a-1", "b-0", "b-1"];
    assert!(
        entries.len() == expected.len() && entries.iter().zip(expected).all(|(entry, msg)| entry.contains(msg)),
        "the entries were not shown in the order {expected:?}: {entries:?}"
    );
}

/// FR-030: a record that reaches the file in two writes is shown as one formatted entry,
/// also when the first write carries enough complete records to reach the maximum message size.
#[rstest]
#[case::alone(0)]
#[case::after_records_reaching_limit(40)]
fn test_shows_record_written_in_parts(#[case] records: usize) {
    let (_dir, files) = files(["a"]);
    let file = &files[0];
    let mut hl = follow(&files, &["--max-message-size", "1 KiB"]);

    let complete = (0..records)
        .map(|i| record("2026-07-06T10:00:00Z", &format!("filler-{i}")))
        .collect::<String>();
    let split = record("2026-07-06T10:00:01Z", "split-record");
    let (head, tail) = split.split_at(split.len() / 2);
    // The pause is longer than the sync interval, so the first part is read and held alone.
    append_parts(
        file,
        &[format!("{complete}{head}").as_str(), tail],
        Duration::from_millis(500),
    );

    let entry = hl.wait_entry("split", TIMEOUT).expect("the split record was not shown");
    assert!(!entry.contains('{'), "the split record was shown as raw text: {entry}");
    assert!(
        entry.contains("10:00:01.000") && entry.contains("split-record"),
        "{entry}"
    );
}

/// FR-029: text that is not a record is shown right after the record before it in its input,
/// also when a record of another input with an earlier timestamp comes at the same time.
#[test]
fn test_shows_text_after_record_before_it() {
    let (_dir, files) = files(["a", "b"]);
    let mut hl = follow(&files, &["--sync-interval-ms", "1000"]);

    append(
        &files[0],
        format!("{}  at stack.frame\n", record("2026-07-06T10:00:02Z", "a-record")),
    );
    append(&files[1], record("2026-07-06T10:00:01Z", "b-record"));

    let entries = hl.next_entries(3, TIMEOUT);
    assert_eq!(entries.len(), 3, "{entries:?}");
    for (entry, text) in entries.iter().zip(["b-record", "a-record", "at stack.frame"]) {
        assert!(entry.contains(text), "expected {text} in this order: {entries:?}");
    }
}

/// FR-031: an entry longer than the maximum message size is shown whole, as one entry in its place,
/// also when it is written in parts and when the timestamps of its input are out of order.
#[rstest]
#[case::whole(1, "2026-07-06T10:00:03Z")]
#[case::in_parts(3, "2026-07-06T10:00:03Z")]
#[case::in_parts_out_of_order(3, "2026-07-06T10:00:00Z")]
fn test_shows_long_entry_whole(#[case] parts: usize, #[case] after: &str) {
    let (_dir, files) = files(["a", "b"]);
    // The pauses between parts are shorter than the sync interval, so the entry is never ended early.
    let mut hl = follow(&files, &["--max-message-size", "1 KiB", "--sync-interval-ms", "1000"]);

    let long = "x".repeat(3000);
    let mut writes = long
        .as_bytes()
        .chunks(long.len().div_ceil(parts))
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();
    writes[0].splice(0..0, record("2026-07-06T10:00:01Z", "before").into_bytes());
    writes
        .last_mut()
        .unwrap()
        .extend(format!("\n{}", record(after, "after")).into_bytes());

    append(&files[0], &writes[0]);
    // A record of the other input comes while the entry is being written.
    append(&files[1], record("2026-07-06T10:00:02Z", "b-record"));
    thread::sleep(Duration::from_millis(200));
    append_parts(&files[0], &writes[1..], Duration::from_millis(200));

    let entries = hl.next_entries(4, TIMEOUT);
    assert_eq!(entries.len(), 4, "{entries:?}");
    // The badges show names of temporary directories, which may contain an `x` too.
    let long_entries = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.contains("xxxxxxxx"))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    assert_eq!(long_entries.len(), 1, "the long entry was split: {entries:?}");
    assert!(
        entries[long_entries[0]].ends_with(&long),
        "the long entry was not shown whole: {entries:?}"
    );
    for msg in ["before", "after", "b-record"] {
        assert!(
            entries.iter().any(|entry| entry.contains(msg) && !entry.contains('{')),
            "{msg} was not shown as a formatted record: {entries:?}"
        );
    }
    assert!(
        long_entries[0] != 0 && entries[long_entries[0] - 1].contains("before"),
        "the long entry was not shown right after the record before it: {entries:?}"
    );
}

/// FR-034: with an output delimiter other than a line break, an entry is written as soon as it is shown.
#[test]
fn test_writes_entries_at_once_with_nul_delimiter() {
    let (_dir, files) = files(["a"]);
    let file = &files[0];
    let mut hl = Hl::spawn_with_delimiter(args(&files, &["--output-delimiter", "nul"]), b'\0');

    append(file, record("2026-07-06T10:00:00Z", "next"));
    assert!(hl.wait_entry("next", TIMEOUT).is_some(), "the record was not written");
}

/// SC-012: a long entry in a followed file is shown in about the time the same data takes from standard input,
/// rather than in a time that grows with the square of its length.
#[test]
fn test_shows_long_entry_quickly() {
    // Before the fix, a debug build took more than 30 s on a laptop, and now it takes about 1 s.
    const LIMIT: Duration = Duration::from_secs(15);

    let (_dir, files) = files(["a"]);
    let file = &files[0];
    let mut hl = follow(&files, &["--max-message-size", "64 MiB"]);

    // The entry is written in parts, so it is read while it is not yet terminated.
    let start = Instant::now();
    let part = "x".repeat(1 << 20);
    // It is longer than the maximum message size.
    for _ in 0..65 {
        append(file, &part);
    }
    append(file, format!("\n{}", record("2026-07-06T10:00:00Z", "after")));
    let shown = hl.wait_entry("after", 2 * LIMIT).is_some();
    let elapsed = start.elapsed();
    assert!(
        shown && elapsed < LIMIT,
        "the long entry was not shown within {LIMIT:?}"
    );
}

/// FR-035, SC-013: once its output is closed, `hl` exits within 1 second, even with no new data to show.
#[test]
fn test_exits_once_output_is_closed() {
    use std::{
        io::{BufRead, BufReader},
        sync::mpsc,
    };

    const LIMIT: Duration = Duration::from_secs(1);

    let (_dir, files) = files(["a"]);
    let (mut hl, stdout) = Hl::spawn_unread(args(&files, &[]));

    // The first entry is read by a thread of its own, which hands the output back, so a test that fails does not hang.
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        let mut entry = String::new();
        stdout.read_line(&mut entry).ok();
        tx.send((entry, stdout)).ok();
    });
    append(&files[0], record("2026-07-06T10:00:00Z", "ready"));
    let (entry, stdout) = rx.recv_timeout(TIMEOUT).expect("the record was not shown");
    assert!(entry.contains("ready"), "unexpected entry {entry:?}");

    let start = Instant::now();
    // Nothing is appended, so the output is found closed without anything written to it.
    drop(stdout);
    let status = hl.wait_exit(|| {});
    let elapsed = start.elapsed();
    assert!(
        status.is_some() && elapsed < LIMIT,
        "hl did not exit within {LIMIT:?} once its output was closed"
    );
}
