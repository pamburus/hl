//! Following directories, devices and other special files.

#[cfg(unix)]
use std::path::Path;
use std::{
    fs::{self, File},
    io,
    time::Duration,
};

use fsmon::{Follow, FollowOptions};
use rstest::rstest;

#[cfg(unix)]
use crate::support::{Event, PATIENCE};
use crate::support::{Reader, append};

#[cfg(unix)]
use unix::*;

#[cfg(unix)]
mod unix {
    use std::{
        ffi::CStr,
        fs::{self, File, OpenOptions},
        io::Write,
        os::{
            fd::FromRawFd,
            unix::fs::{FileTypeExt, OpenOptionsExt},
        },
        path::PathBuf,
        time::{Duration, SystemTime},
    };

    /// Opens a pseudo-terminal, returning its master, its slave, which keeps it open, and the path of its slave.
    pub fn pty() -> (File, File, PathBuf) {
        // SAFETY: the flags are valid.
        let fd = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
        assert!(fd >= 0, "posix_openpt failed");
        // SAFETY: `fd` is an open descriptor that nothing else owns.
        let master = unsafe { File::from_raw_fd(fd) };
        // SAFETY: `fd` is the master of a pseudo-terminal.
        assert_eq!(unsafe { libc::grantpt(fd) }, 0);
        // SAFETY: as above.
        assert_eq!(unsafe { libc::unlockpt(fd) }, 0);
        // SAFETY: as above. The name it returns is copied at once, and no other test calls it.
        let name = unsafe { libc::ptsname(fd) };
        assert!(!name.is_null(), "ptsname failed");
        // SAFETY: `name` is a NUL-terminated string.
        let path = PathBuf::from(unsafe { CStr::from_ptr(name) }.to_str().unwrap());
        let slave = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOCTTY)
            .open(&path)
            .unwrap();
        (master, slave, path)
    }

    /// Types `text` into the terminal of `master`.
    pub fn type_into(mut master: &File, text: &str) {
        master.write_all(text.as_bytes()).unwrap();
    }

    /// Changes the modification time of `file`.
    pub fn touch(file: &File) {
        let later = SystemTime::now() + Duration::from_secs(1);
        file.set_times(fs::FileTimes::new().set_modified(later)).unwrap();
    }

    /// Returns a block device, if there is one.
    pub fn block_device() -> Option<PathBuf> {
        fs::read_dir("/dev")
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| fs::metadata(path).is_ok_and(|meta| meta.file_type().is_block_device()))
    }
}

// ---

/// FR-026: a directory is waited for, as with `tail -F`, and the file that takes its place is read from its start.
#[rstest]
fn test_directory_is_waited_for_until_a_file_takes_its_place(#[values(false, true)] with_handle: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    fs::create_dir(&path).unwrap();
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let input = if with_handle {
        match File::open(&path) {
            Ok(file) => follow.add_file(&path, file).unwrap(),
            // A directory cannot be opened as a file on some systems.
            Err(_) => return,
        }
    } else {
        follow.add_path(&path).unwrap()
    };
    let mut reader = Reader::spawn(&follow, input);
    reader.expect_quiet(Duration::from_millis(100));

    fs::remove_dir(&path).unwrap();
    append(&path, "first\n");
    reader.expect_data("first\n");
}

/// FR-026: a directory is rejected when paths that cannot be opened are not waited for.
#[rstest]
fn test_directory_is_rejected_when_not_retrying(#[values(false, true)] with_handle: bool) {
    let dir = tempfile::tempdir().unwrap();
    let options = FollowOptions {
        retry_missing: false,
        ..Default::default()
    };
    let follow = Follow::new(options).unwrap();
    let result = if with_handle {
        match File::open(dir.path()) {
            Ok(file) => follow.add_file(dir.path(), file),
            // A directory cannot be opened as a file on some systems.
            Err(_) => return,
        }
    } else {
        follow.add_path(dir.path())
    };
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::InvalidInput);
}

/// FR-026: a character device, such as `/dev/null`, is followed as with `tail -F`, which reads nothing from it and
/// does not end its stream.
#[cfg(unix)]
#[rstest]
fn test_character_device_is_followed(#[values(false, true)] with_handle: bool) {
    let path = Path::new("/dev/null");
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let input = if with_handle {
        follow.add_file(path, File::open(path).unwrap()).unwrap()
    } else {
        follow.add_path(path).unwrap()
    };
    let reader = Reader::spawn(&follow, input);
    reader.expect_quiet(Duration::from_millis(300));
}

/// FR-026: what can be read from a character device is read again each time its modification time changes, as with
/// `tail -F`, and only then, so what is typed into a terminal, which does not change it, is left unread until then.
#[cfg(unix)]
#[test]
fn test_character_device_is_read_again_once_modified() {
    let (master, slave, path) = pty();
    let options = FollowOptions {
        poll_interval: Duration::from_millis(50),
        ..Default::default()
    };
    let follow = Follow::new(options).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    type_into(&master, "typed\n");
    reader.expect_quiet(Duration::from_millis(300));
    touch(&slave);
    reader.expect_data("typed\n");
}

/// FR-026: a character device is followed until its path refers to another file, which is then read from its start.
#[cfg(unix)]
#[test]
fn test_character_device_is_followed_until_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    std::os::unix::fs::symlink("/dev/null", &path).unwrap();
    let options = FollowOptions {
        poll_interval: Duration::from_millis(50),
        recheck_cadence: Duration::from_millis(100),
        ..Default::default()
    };
    let follow = Follow::new(options).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    reader.expect_quiet(Duration::from_millis(200));

    fs::remove_file(&path).unwrap();
    append(&path, "first\n");
    reader.expect(Event::End, PATIENCE);
    reader.expect_data("first\n");
}

/// FR-026: a special file given with a handle that cannot be followed, such as a socket, is waited for, as with
/// `tail -F`, and the file that then appears at its path is read from its start.
#[cfg(unix)]
#[test]
fn test_socket_given_with_a_handle_is_waited_for() {
    use std::os::{fd::OwnedFd, unix::net::UnixStream};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    let (socket, _peer) = UnixStream::pair().unwrap();
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let input = follow.add_file(&path, File::from(OwnedFd::from(socket))).unwrap();
    let mut reader = Reader::spawn(&follow, input);
    reader.expect_quiet(Duration::from_millis(100));

    append(&path, "first\n");
    reader.expect_data("first\n");
}

/// FR-026: a block device is waited for, as with `tail -F`, and the file that takes its place is read from its start.
#[cfg(unix)]
#[test]
fn test_block_device_is_waited_for_until_a_file_takes_its_place() {
    let Some(device) = block_device() else {
        // There may be no block devices, as in a container.
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    std::os::unix::fs::symlink(device, &path).unwrap();
    let follow = Follow::new(FollowOptions::default()).unwrap();
    let mut reader = Reader::spawn(&follow, follow.add_path(&path).unwrap());
    reader.expect_quiet(Duration::from_millis(100));

    fs::remove_file(&path).unwrap();
    append(&path, "first\n");
    reader.expect_data("first\n");
}
