//! Following FIFOs and pipes.
#![cfg(unix)]

use std::{
    fs::OpenOptions,
    io::{Read, Write},
    os::fd::AsRawFd,
    time::Duration,
};

use fsmon::{Follow, FollowOptions};
use rstest::rstest;

use crate::support::{Event, PATIENCE, Reader, interrupt_for_a_while, mkfifo, path_of, pipe};

/// FR-002, FR-026: a FIFO that appears at a path waited for is read from its first writer.
#[test]
fn test_fifo_that_appears_at_missing_path_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.fifo");
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    mkfifo(&path);
    // The open waits for the FIFO to be opened by the follow, so it is made by a thread of its own, which a test that
    // fails does not wait for.
    let writer = path.clone();
    std::thread::spawn(move || {
        let mut writer = OpenOptions::new().write(true).open(writer).unwrap();
        writer.write_all(b"first\n").unwrap();
    });
    reader.expect_data("first\n");
    reader.expect(Event::End, PATIENCE);
}

/// FR-026: each writer of a FIFO, one after another, makes a stream, which ends with what the writer left unterminated,
/// and the next writer is waited for.
#[rstest]
fn test_fifo_makes_a_stream_per_writer(#[values(false, true)] with_handle: bool) {
    use std::os::unix::fs::OpenOptionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.fifo");
    mkfifo(&path);
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let input = if with_handle {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&path)
            .unwrap();
        follow.add_file(&path, file).unwrap()
    } else {
        follow.add_path(&path).unwrap()
    };
    let mut reader = Reader::spawn(&follow, input);
    reader.expect_quiet(Duration::from_millis(100));

    for data in ["first\nunterminated", "second\n"] {
        // The open waits for the reader to open the FIFO again.
        let mut writer = OpenOptions::new().write(true).open(&path).unwrap();
        writer.write_all(data.as_bytes()).unwrap();
        reader.expect_data(data);
        reader.expect_quiet(Duration::from_millis(50));
        drop(writer);
        reader.expect(Event::End, PATIENCE);
    }
}

/// FR-026, FR-036: a signal that interrupts the wait for a writer of a FIFO does not end the stream, which then reads
/// what the writer writes.
#[test]
fn test_fifo_wait_resumes_after_interruption() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.fifo");
    mkfifo(&path);
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let input = follow.add_path(&path).unwrap();
    let mut reader = Reader::spawn(&follow, input);

    interrupt_for_a_while(reader.thread());
    reader.expect_quiet(Duration::from_millis(50));

    let mut writer = OpenOptions::new().write(true).open(&path).unwrap();
    writer.write_all(b"first\n").unwrap();
    reader.expect_data("first\n");
}

/// FR-026: the stream of a FIFO writer ends also when the writer closes the FIFO before its data is read.
#[test]
fn test_fifo_stream_ends_when_writer_closes_before_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.fifo");
    mkfifo(&path);
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let input = follow.add_path(&path).unwrap();

    let mut writer = OpenOptions::new().write(true).open(&path).unwrap();
    writer.write_all(b"first\nunterminated").unwrap();
    drop(writer);

    let mut reader = Reader::spawn(&follow, input);
    reader.expect_data("first\nunterminated");
    reader.expect(Event::End, PATIENCE);
}

/// FR-026: a writer that opens a FIFO as the stream of the previous writer ends neither gets an error nor loses what it
/// writes, whether it writes before or after the stream is closed.
#[rstest]
fn test_fifo_writer_between_streams_is_read(#[values(false, true)] write_before_close: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.fifo");
    mkfifo(&path);
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let mut input = follow.add_path(&path).unwrap();
    let mut stream = input.next_stream().unwrap().unwrap();

    let mut writer = OpenOptions::new().write(true).open(&path).unwrap();
    writer.write_all(b"first\n").unwrap();
    drop(writer);
    let mut data = String::new();
    stream.read_to_string(&mut data).unwrap();
    assert_eq!(data, "first\n");

    // The open does not wait, as the FIFO is still open by the stream that ended.
    let mut writer = OpenOptions::new().write(true).open(&path).unwrap();
    if write_before_close {
        writer.write_all(b"second\n").unwrap();
        // What the next writer writes is left for the next stream.
        assert_eq!(stream.read(&mut [0; 16]).unwrap(), 0);
        drop(stream);
    } else {
        drop(stream);
        writer.write_all(b"second\n").unwrap();
    }
    drop(writer);

    let mut reader = Reader::spawn(&follow, input);
    reader.expect_data("second\n");
    reader.expect(Event::End, PATIENCE);
}

/// FR-026: a pipe that has no name, such as one given by process substitution, is read to its end, where following it
/// stops, as it can have no other writer.
#[rstest]
fn test_unnamed_pipe_is_read_to_its_end(#[values(false, true)] by_path: bool) {
    // The read end is kept open, as the shell keeps the one it gives for process substitution, so that its path can be
    // opened again.
    let (read, mut write) = pipe();
    let path = path_of(&read);
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let input = if by_path {
        follow.add_path(&path).unwrap()
    } else {
        follow.add_file(&path, read.try_clone().unwrap()).unwrap()
    };

    write.write_all(b"only\n").unwrap();
    drop(write);
    let mut reader = Reader::spawn(&follow, input);
    reader.expect_data("only\n");
    reader.expect(Event::End, PATIENCE);
    reader.expect(Event::Done, PATIENCE);
    drop(read);
}

/// FR-026: a FIFO given with a handle is left blocking, as the state of the handle may be shared with other processes,
/// such as the standard input given by its path, which reads like a FIFO, and would fail to be read by them otherwise.
#[test]
fn test_fifo_handle_is_left_blocking() {
    use std::os::unix::fs::OpenOptionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.fifo");
    mkfifo(&path);
    // Opened without blocking, as an open for reading waits for a writer otherwise, then made blocking.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)
        .unwrap();
    // SAFETY: the descriptor is open.
    assert_eq!(unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, 0) }, 0);
    let shared = file.try_clone().unwrap();
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_file(&path, file).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    let mut writer = OpenOptions::new().write(true).open(&path).unwrap();
    writer.write_all(b"first\n").unwrap();
    reader.expect_data("first\n");
    drop(writer);
    reader.expect(Event::End, PATIENCE);
    // SAFETY: the descriptor is open.
    let flags = unsafe { libc::fcntl(shared.as_raw_fd(), libc::F_GETFL) };
    assert_eq!(flags & libc::O_NONBLOCK, 0);
}
