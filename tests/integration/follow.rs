//! Follow mode on files.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::support::{hl::Hl, log::append_record, temp::TempDir};

/// Starts following `app.log` in `dir`, created with a record, and waits for the record to be shown.
fn follow(dir: &TempDir) -> (PathBuf, Hl) {
    let file = dir.join("app.log");
    fs::write(&file, b"").unwrap();
    append_record(&file, "first");

    let mut hl = Hl::spawn([OsStr::new("--follow"), file.as_os_str()]);
    let deadline = Instant::now() + Duration::from_secs(10);
    assert!(hl.wait_line("first", deadline), "the record in the file was not shown");
    (file, hl)
}

/// Appends records with messages starting with `prefix` to `file` until one of them is shown.
///
/// A change may be seen only some time after it is made, so a record is appended again until it is.
fn append_until_shown(hl: &mut Hl, file: &Path, prefix: &str) {
    let shown = (0..100).any(|i| {
        append_record(file, &format!("{prefix}-{i}"));
        hl.wait_line(&format!("{prefix}-"), Instant::now() + Duration::from_millis(100))
    });
    assert!(shown, "no record appended as {prefix} was shown");
}

/// Appends records to `file` until it is watched.
///
/// The first record may be shown by the initial read, so the second one is shown because of the watch.
fn wait_watched(hl: &mut Hl, file: &Path) {
    append_until_shown(hl, file, "appended-0");
    append_until_shown(hl, file, "appended-1");
}

/// Closes the output of `hl` and checks that it exits successfully.
fn stop(mut hl: Hl, file: &Path) {
    let status = hl.stop(|| append_record(file, "stop"));
    let status = status.expect("hl did not exit after its output was closed");
    assert!(status.success(), "hl exited with {status}");
}

#[test]
fn test_follow_shows_records_appended_to_file() {
    // Created before the process, so it is removed after the process has stopped.
    let dir = TempDir::new("follow-append");
    let (file, mut hl) = follow(&dir);

    wait_watched(&mut hl, &file);

    stop(hl, &file);
}

#[test]
fn test_follow_shows_records_of_file_replacing_rotated_one() {
    let dir = TempDir::new("follow-rotate");
    let (file, mut hl) = follow(&dir);
    wait_watched(&mut hl, &file);

    fs::rename(&file, dir.join("app.log.1")).unwrap();
    fs::write(&file, b"").unwrap();
    append_until_shown(&mut hl, &file, "rotated");

    stop(hl, &file);
}

#[test]
fn test_follow_shows_records_of_file_created_again_after_removal() {
    let dir = TempDir::new("follow-recreate");
    let (file, mut hl) = follow(&dir);
    wait_watched(&mut hl, &file);

    fs::remove_file(&file).unwrap();
    fs::write(&file, b"").unwrap();
    append_until_shown(&mut hl, &file, "recreated");

    stop(hl, &file);
}
