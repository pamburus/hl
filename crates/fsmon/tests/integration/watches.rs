//! Waiting for changes with native notifications of the operating system.
//!
//! Each test checks again only after a long time, so that what it expects is seen only if a notification comes.

use std::time::Duration;

use fsmon::{Follow, FollowOptions};

use crate::support::{Reader, append};

/// Options with which nothing is checked again for a long time, so that only a notification ends a wait.
fn long_waits() -> FollowOptions {
    FollowOptions {
        poll_interval: Duration::from_secs(60),
        recheck_cadence: Duration::from_secs(60),
        ..Default::default()
    }
}

/// Returns the number of inotify instances the process has open.
#[cfg(target_os = "linux")]
fn inotify_instances() -> usize {
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(|entry| std::fs::read_link(entry.ok()?.path()).ok())
        .filter(|target| target == std::path::Path::new("anon_inode:inotify"))
        .count()
}

// ---

/// FR-037: the watches of all followed paths share one inotify instance, so that following many files does not use up
/// the instances a user may have, which are 128 by default.
#[cfg(target_os = "linux")]
#[test]
fn test_watches_share_one_inotify_instance() {
    let dir = tempfile::tempdir().unwrap();
    let follow = Follow::new(long_waits()).unwrap();
    let readers: Vec<_> = (0..4)
        .map(|i| Reader::spawn(&follow, follow.add_path(dir.path().join(format!("{i}.log"))).unwrap()))
        .collect();
    readers[0].expect_quiet(Duration::from_millis(200));
    assert_eq!(inotify_instances(), 1);
}

/// FR-010: a change is seen by the input of the changed path, also when another input that waits in the same directory
/// is told of it first, which happens to one change or another of several.
#[test]
fn test_change_wakes_the_input_of_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.log"), dir.path().join("b.log"));
    append(&a, "");
    append(&b, "");
    let follow = Follow::new(long_waits()).unwrap();
    let other = Reader::spawn(&follow, follow.add_path(&b).unwrap());
    let mut reader = Reader::spawn(&follow, follow.add_path(&a).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    for line in ["1\n", "2\n", "3\n", "4\n", "5\n", "6\n", "7\n", "8\n"] {
        append(&a, line);
        reader.expect_data(line);
    }
    other.expect_quiet(Duration::from_millis(100));
}

/// FR-010: once another follow of a path in the same directory ends, changes in the directory are still seen.
#[test]
fn test_ended_follow_keeps_the_watches_of_its_directory() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.log"), dir.path().join("b.log"));
    append(&a, "");
    append(&b, "");
    let follow = Follow::new(long_waits()).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(&a).unwrap());
    let other = Follow::new(long_waits()).unwrap();
    let ended = Reader::spawn(&other, other.add_path(&b).unwrap());
    ended.expect_quiet(Duration::from_millis(100));
    drop(ended);

    append(&a, "new\n");
    reader.expect_data("new\n");
}

/// Edge case "Symlinked target": a link whose target is missing is waited for, and the file that appears at its target
/// is read from its start.
#[cfg(unix)]
#[test]
fn test_dangling_link_is_followed_once_its_target_appears() {
    let dir = tempfile::tempdir().unwrap();
    let (target, link) = (dir.path().join("app.log"), dir.path().join("current.log"));
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let follow = Follow::new(long_waits()).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(&link).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    append(&target, "first\n");
    reader.expect_data("first\n");
}

/// Edge case "Symlinked target": once a link that a followed link leads through is pointed at another file, the file
/// it then leads to is read from its start.
#[cfg(unix)]
#[test]
fn test_link_is_followed_once_a_link_it_leads_through_is_pointed_elsewhere() {
    use std::{fs, os::unix::fs::symlink};

    use crate::support::{Event, PATIENCE};

    let dir = tempfile::tempdir().unwrap();
    let path = |name: &str| dir.path().join(name);
    append(&path("app-1.log"), "");
    append(&path("app-2.log"), "next\n");
    symlink(path("app-1.log"), path("app.log")).unwrap();
    symlink(path("app.log"), path("current.log")).unwrap();
    let follow = Follow::new(long_waits()).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(path("current.log")).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    symlink(path("app-2.log"), path("app.log.new")).unwrap();
    fs::rename(path("app.log.new"), path("app.log")).unwrap();
    reader.expect(Event::End, PATIENCE);
    reader.expect_data("next\n");
}
