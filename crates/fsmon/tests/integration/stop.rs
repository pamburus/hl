//! Stopping a follow.

use std::time::Duration;

use fsmon::{Follow, FollowOptions};
use rstest::rstest;

use crate::support::{Event, Mode, Reader, append, follow};

/// Options with which nothing is checked again for a long time, so that only a stop ends a wait.
fn long_waits() -> FollowOptions {
    FollowOptions {
        poll_interval: Duration::from_secs(60),
        recheck_cadence: Duration::from_secs(60),
        ..Default::default()
    }
}

// ---

/// FR-036: a stop ends the reads that wait for data at once, however long the wait would be, for every input.
#[rstest]
fn test_stop_ends_waiting_reads(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let follow = follow(mode, long_waits());
    let mut readers = ["a.log", "b.log"].map(|name| {
        let path = dir.path().join(name);
        append(&path, "");
        Reader::spawn(&follow, follow.add_path(&path).unwrap())
    });
    readers[0].expect_quiet(Duration::from_millis(200));

    follow.stopper().stop();
    for reader in &mut readers {
        reader.expect(Event::End, Duration::from_secs(1));
        reader.expect(Event::Done, Duration::from_secs(1));
    }
}

/// FR-036: a stop ends a wait for a missing path at once.
#[rstest]
fn test_stop_ends_a_wait_for_missing_path(#[values(Mode::Native, Mode::Polled)] mode: Mode) {
    let dir = tempfile::tempdir().unwrap();
    let follow = follow(mode, long_waits());
    let mut reader = Reader::spawn(&follow, follow.add_path(dir.path().join("app.log")).unwrap());
    reader.expect_quiet(Duration::from_millis(200));

    follow.stopper().stop();
    reader.expect(Event::Done, Duration::from_secs(1));
}

/// FR-036: once stopped, an input delivers no more streams, whether its path is there or still waited for.
#[rstest]
fn test_stopped_input_has_no_streams(#[values(false, true)] exists: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    if exists {
        append(&path, "");
    }
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let mut input = follow.add_path(&path).unwrap();
    follow.stopper().stop();
    assert!(input.next_stream().unwrap().is_none());
}

/// FR-036: a stop ends a read of a FIFO with no writer at once.
#[cfg(unix)]
#[test]
fn test_stop_ends_a_read_of_fifo() {
    use crate::support::mkfifo;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.fifo");
    mkfifo(&path);
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    reader.expect_quiet(Duration::from_millis(200));

    follow.stopper().stop();
    reader.expect(Event::End, Duration::from_secs(1));
    reader.expect(Event::Done, Duration::from_secs(1));
}

/// FR-036: a stop ends a read of a pipe that has no name whose writer writes nothing at once, also when the pipe is
/// opened again by its path, which gives the descriptor of the pipe itself on macOS, whatever flags it is opened with.
#[cfg(unix)]
#[rstest]
fn test_stop_ends_a_read_of_unnamed_pipe(#[values(false, true)] by_path: bool) {
    use crate::support::{path_of, pipe};

    let (read, write) = pipe();
    let path = path_of(&read);
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let input = if by_path {
        follow.add_path(&path).unwrap()
    } else {
        follow.add_file(&path, read.try_clone().unwrap()).unwrap()
    };
    let mut reader = Reader::spawn(&follow, input);
    reader.expect_quiet(Duration::from_millis(200));

    follow.stopper().stop();
    reader.expect(Event::End, Duration::from_secs(1));
    drop((read, write));
}

/// FR-036: a stop ends a read of a pipe whose writer writes nothing at once, also on Windows, where a pipe cannot be
/// waited on to become readable.
#[cfg(windows)]
#[test]
fn test_stop_ends_a_read_of_pipe() {
    use std::{fs::File, os::windows::io::OwnedHandle};

    let (read, write) = std::io::pipe().unwrap();
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let input = follow.add_file("pipe", File::from(OwnedHandle::from(read))).unwrap();
    let mut reader = Reader::spawn(&follow, input);
    reader.expect_quiet(Duration::from_millis(200));

    follow.stopper().stop();
    reader.expect(Event::End, Duration::from_secs(1));
    drop(write);
}
