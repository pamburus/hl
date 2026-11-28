//! Following regular files.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    time::Duration,
};

use fsmon::{FallbackPolicy, Follow, FollowOptions};
use rstest::rstest;

use crate::support::{Event, Mode, PATIENCE, Reader, append, follow};

// ---

/// FR-001: a followed file is read from its end, and what is appended to it is read as it comes.
#[rstest]
fn test_appended_data_is_read(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "old\n");
    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());

    append(&path, "new\n");
    reader.expect_data("new\n");
    append(&path, "partial");
    reader.expect_data("partial");
}

/// FR-001: a recheck cadence too long to set a deadline for, such as `Duration::MAX`, is waited for with no deadline, and
/// what is appended is still read.
#[test]
fn test_appended_data_is_read_with_recheck_cadence_too_long_for_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "old\n");
    let options = FollowOptions {
        recheck_cadence: Duration::MAX,
        ..FollowOptions::default()
    };
    let follow = follow(Mode::Native, options);
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    append(&path, "new\n");
    reader.expect_data("new\n");
}

/// A read into an empty buffer reads nothing and leaves the stream as it was, as `Read` requires.
#[test]
fn test_read_into_empty_buffer_reads_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "first\n");
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let mut stream = follow
        .add_path_from_start(&path)
        .unwrap()
        .next_stream()
        .unwrap()
        .unwrap();

    assert_eq!(stream.read(&mut []).unwrap(), 0);
    let mut buf = [0; 16];
    let n = stream.read(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"first\n");
}

/// FR-036: a signal that interrupts the wait of a stream for data ends neither the stream nor its read, which then
/// reads what is appended.
#[cfg(unix)]
#[rstest]
fn test_read_resumes_after_interruption(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    use crate::support::interrupt_for_a_while;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "");
    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    interrupt_for_a_while(reader.thread());
    reader.expect_quiet(Duration::from_millis(50));
    append(&path, "new\n");
    reader.expect_data("new\n");
}

/// FR-002, FR-023: with the optimistic fallback policy, a path whose directory is missing is waited for, and the file
/// that appears at it once the directory is created is read from its start.
#[test]
fn test_optimistic_policy_waits_for_missing_directory() {
    let dir = tempfile::tempdir().unwrap();
    let logs = dir.path().join("logs");
    let path = logs.join("app.log");
    let options = FollowOptions {
        fallback_policy: FallbackPolicy::Optimistic,
        poll_interval: Duration::from_millis(50),
        ..Default::default()
    };
    let follow = Follow::new(options).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());

    reader.expect_quiet(Duration::from_millis(100));
    fs::create_dir(&logs).unwrap();
    append(&path, "first\n");
    reader.expect_data("first\n");
}

/// FR-002: a missing path is waited for, and the file that appears at it is read from its start.
#[rstest]
fn test_missing_path_is_waited_for(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());

    reader.expect_quiet(Duration::from_millis(100));
    append(&path, "first\n");
    reader.expect_data("first\n");
}

/// FR-002: a path added to be read from its start is read from the start of the file, whether the file is there
/// already, as when it appeared since it was found missing, or appears later.
#[rstest]
fn test_path_added_from_start_is_read_from_start(
    #[values(Mode::Native, Mode::Polled)] mode: Mode,
    #[values(false, true)] exists: bool,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    if exists {
        append(&path, "first\n");
    }
    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_path_from_start(&path).unwrap());

    if !exists {
        reader.expect_quiet(Duration::from_millis(100));
        append(&path, "first\n");
    }
    reader.expect_data("first\n");
    append(&path, "next\n");
    reader.expect_data("next\n");
}

/// FR-002: without retries, a missing path is an error.
#[test]
fn test_missing_path_without_retries_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let options = FollowOptions {
        retry_missing: false,
        ..Default::default()
    };
    let follow = Follow::new(options).unwrap();
    let err = follow.add_path(dir.path().join("app.log")).err().unwrap();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}

/// FR-002, FR-023: without retries, an input ends once its path refers to no file that can be followed.
#[rstest]
#[case::removed(false)]
#[case::replaced_by_directory(true)]
fn test_input_without_retries_ends_once_file_is_gone(#[case] directory: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "");
    let options = FollowOptions {
        retry_missing: false,
        ..Default::default()
    };
    let follow = Follow::new(options).unwrap();
    let input = follow.add_path(&path).unwrap();

    // The directory is in place before the input is read, so that it is not taken for a missing path. On Windows, a
    // file removed while it is open may keep its name until it is closed, which a rename does not.
    if directory {
        fs::rename(&path, dir.path().join("app.log.1")).unwrap();
        fs::create_dir(&path).unwrap();
    } else {
        fs::remove_file(&path).unwrap();
    }
    let mut reader = Reader::spawn(&follow, input);
    reader.expect(Event::End, PATIENCE);
    reader.expect(Event::Done, PATIENCE);
}

/// FR-023: with the optimistic fallback policy, a path is followed as with the default one.
#[test]
fn test_optimistic_policy_follows_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "");
    let options = FollowOptions {
        fallback_policy: FallbackPolicy::Optimistic,
        ..Default::default()
    };
    let follow = Follow::new(options).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());

    append(&path, "new\n");
    reader.expect_data("new\n");
}

/// FR-004: a truncated file ends its stream, and the next one reads it from its start.
#[rstest]
fn test_truncation_starts_a_new_stream(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "old\n");
    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    append(&path, "before\n");
    reader.expect_data("before\n");

    OpenOptions::new().write(true).open(&path).unwrap().set_len(0).unwrap();
    append(&path, "after\n");
    reader.expect(Event::End, PATIENCE);
    reader.expect_data("after\n");
}

/// FR-005: a rotated file is read to its end, and the file put in its place is read from its start.
#[rstest]
fn test_rotation_starts_a_new_stream(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "");
    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    append(&path, "a\n");
    reader.expect_data("a\n");

    append(&path, "last\n");
    fs::rename(&path, dir.path().join("app.log.1")).unwrap();
    append(&path, "next\n");
    reader.expect_data("last\n");
    reader.expect(Event::End, PATIENCE);
    reader.expect_data("next\n");
}

/// FR-006: a file replaced by one of the same size is noticed by the recheck.
#[rstest]
fn test_replacement_by_same_size_file_starts_a_new_stream(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "aaaa\n");
    let options = FollowOptions {
        recheck_cadence: Duration::from_millis(200),
        ..Default::default()
    };
    let follow = follow(mode, options);
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    let next = dir.path().join("next.log");
    append(&next, "bbbb\n");
    fs::rename(&next, &path).unwrap();
    reader.expect(Event::End, PATIENCE);
    reader.expect_data("bbbb\n");
}

/// FR-007: a removed file ends its stream, and a file created at its path later is read from its start.
#[rstest]
fn test_removal_ends_the_stream(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "");
    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());

    fs::remove_file(&path).unwrap();
    reader.expect(Event::End, PATIENCE);
    reader.expect_quiet(Duration::from_millis(100));
    append(&path, "again\n");
    reader.expect_data("again\n");
}

/// FR-008: a file added with its handle is read from the position of the handle, and its identity is that of the
/// handle, so a replacement made before it was added still ends its stream.
#[rstest]
fn test_file_added_with_handle_starts_at_its_position(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "skipped\nkept\n");
    let mut file = File::open(&path).unwrap();
    file.seek(SeekFrom::Start(8)).unwrap();
    fs::rename(&path, dir.path().join("app.log.1")).unwrap();
    append(&path, "next\n");

    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_file(&path, file).unwrap());
    reader.expect_data("kept\n");
    reader.expect(Event::End, PATIENCE);
    reader.expect_data("next\n");
}

/// FR-002, FR-008: a file added with its handle while its directory is missing, as when the whole directory is
/// rotated, is followed again once the directory and a file in it are back.
///
/// Not on Windows, where a directory cannot be renamed while a file in it is open.
#[cfg(unix)]
#[rstest]
fn test_file_added_while_its_directory_is_missing_is_followed_once_back(
    #[values(Mode::Native, Mode::Polled)] mode: Mode,
) {
    let dir = tempfile::tempdir().unwrap();
    let logs = dir.path().join("logs");
    fs::create_dir(&logs).unwrap();
    let path = logs.join("app.log");
    append(&path, "");
    let file = File::open(&path).unwrap();
    fs::rename(&logs, dir.path().join("logs.old")).unwrap();

    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_file(&path, file).unwrap());
    reader.expect(Event::End, PATIENCE);
    fs::create_dir(&logs).unwrap();
    append(&path, "new\n");
    reader.expect_data("new\n");
}

/// FR-002, FR-008, edge case "Symlinked target": a link added with a handle on its target while the target is
/// missing, as when the target is rotated away, is followed to the file that then appears at the target.
#[cfg(unix)]
#[rstest]
fn test_link_added_while_its_target_is_missing_is_followed_once_back(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("app.log");
    let link = dir.path().join("current.log");
    append(&target, "");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let file = File::open(&link).unwrap();
    fs::remove_file(&target).unwrap();

    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_file(&link, file).unwrap());
    reader.expect(Event::End, PATIENCE);
    append(&target, "new\n");
    reader.expect_data("new\n");
}

/// FR-002, FR-008: a file added with its handle once its path is taken by a directory is followed again once a file is
/// back at the path.
#[rstest]
fn test_file_added_while_a_directory_is_at_its_path_is_followed_once_back(
    #[values(Mode::Native, Mode::Polled)] mode: Mode,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "");
    let file = File::open(&path).unwrap();
    // On Windows, a file removed while it is open may keep its name until it is closed, which a rename does not.
    fs::rename(&path, dir.path().join("app.log.1")).unwrap();
    fs::create_dir(&path).unwrap();

    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_file(&path, file).unwrap());
    reader.expect(Event::End, PATIENCE);
    fs::remove_dir(&path).unwrap();
    append(&path, "new\n");
    reader.expect_data("new\n");
}

/// FR-002, FR-008: a file added with its handle once its path is taken by a file that cannot be read is followed
/// again once that file can be read.
#[cfg(unix)]
#[rstest]
fn test_file_added_while_its_path_is_unreadable_is_followed_once_readable(
    #[values(Mode::Native, Mode::Polled)] mode: Mode,
) {
    use std::os::unix::fs::PermissionsExt;
    // SAFETY: the call takes no arguments.
    if unsafe { libc::geteuid() } == 0 {
        // Permissions do not keep the superuser from reading a file.
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    append(&path, "");
    let file = File::open(&path).unwrap();
    fs::remove_file(&path).unwrap();
    append(&path, "new\n");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

    let follow = follow(mode, FollowOptions::default());
    let mut reader = Reader::spawn(&follow, follow.add_file(&path, file).unwrap());
    reader.expect(Event::End, PATIENCE);
    reader.expect_quiet(Duration::from_millis(100));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    reader.expect_data("new\n");
}

/// FR-010: a file of a kernel file system, such as procfs or sysfs, is polled, as it changes with no event that native
/// notifications see, while a file next to the test is watched natively.
#[cfg(target_os = "linux")]
#[rstest]
#[case::procfs("/proc/version")]
#[case::sysfs("/sys/kernel/uevent_seqnum")]
fn test_file_of_kernel_file_system_is_polled(#[case] path: &str) {
    use crate::support::debug_log;

    debug_log();
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().join("app.log");
    append(&local, "");
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let readers = [std::path::Path::new(path), &local]
        .map(|path| Reader::spawn(&follow, follow.add_file(path, File::open(path).unwrap()).unwrap()));
    readers[1].expect_quiet(Duration::from_millis(200));

    let polled = |path: &std::path::Path| {
        let line = format!("fsmon: polling {}", path.display());
        debug_log().iter().any(|logged| logged.starts_with(&line))
    };
    assert!(polled(std::path::Path::new(path)), "{path} was not polled");
    assert!(!polled(&local), "{} was polled", local.display());
}

/// FR-004: a file whose size stays zero while it has content, such as one in procfs, is read again once per check, as
/// with `tail -F`, and not again and again with no pause.
#[cfg(target_os = "linux")]
#[rstest]
fn test_file_of_zero_size_is_read_again_once_per_check(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let path = std::path::Path::new("/proc/version");
    let content = fs::read_to_string(path).unwrap();
    let options = FollowOptions {
        poll_interval: Duration::from_secs(1),
        ..FollowOptions::default()
    };
    let follow = follow(mode, options);
    let file = File::open(path).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_file(path, file).unwrap());
    reader.expect_data(&content);
    reader.expect_quiet(Duration::from_millis(500));
    reader.expect(Event::End, PATIENCE);
    reader.expect_data(&content);
}

/// FR-005, edge case "Symlinked target": a link to a file whose size is larger than its content, such as one in sysfs,
/// is followed to the file it is pointed at, as with `tail -F`.
#[cfg(target_os = "linux")]
#[test]
fn test_link_to_file_larger_than_its_content_is_followed_once_pointed_elsewhere() {
    // Its content, the CPUs that may be present, stays the same while the system runs.
    let target = std::path::Path::new("/sys/devices/system/cpu/possible");
    let content = fs::read_to_string(target).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("current.log");
    std::os::unix::fs::symlink(target, &link).unwrap();
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let file = File::open(&link).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_file(&link, file).unwrap());
    reader.expect_data(&content);

    let other = dir.path().join("app.log");
    append(&other, "new\n");
    let next = dir.path().join("next.log");
    std::os::unix::fs::symlink(&other, &next).unwrap();
    fs::rename(&next, &link).unwrap();
    reader.expect(Event::End, PATIENCE);
    reader.expect_data("new\n");
}
