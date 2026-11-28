//! Filtering entries without `--follow`.

use std::{ffi::OsStr, time::Duration};

use rstest::rstest;
use tempfile::TempDir;

use crate::support::{
    hl::Hl,
    log::{append, record},
};

/// With a filter or `--input-format` set, an entry longer than `--max-message-size` is hidden
/// like other text that is not a record, and without them it is shown.
///
/// No spec covers this yet, see issue 1776.
#[rstest]
#[case::filter(&["--filter", "level=info"], false)]
#[case::input_format(&["--input-format", "json"], false)]
#[case::neither(&[], true)]
fn test_long_entry_hidden_when_filtering(#[case] args: &[&str], #[case] shown: bool) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("a.log");
    let long = "x".repeat(3000);
    append(
        &file,
        format!(
            "{}{long}\n{}",
            record("2026-07-06T10:00:00Z", "before"),
            record("2026-07-06T10:00:01Z", "after"),
        ),
    );

    let mut hl = Hl::spawn(
        ["--max-message-size", "1 KiB"]
            .iter()
            .chain(args)
            .map(|arg| OsStr::new(*arg))
            .chain([file.as_os_str()]),
    );
    let (entries, status) = hl.wait_output(Duration::from_secs(10));

    assert!(status.success(), "hl exited with {status}");
    assert!(
        entries.first().is_some_and(|entry| entry.contains("before")),
        "{entries:?}"
    );
    assert!(
        entries
            .last()
            .is_some_and(|entry| entry.contains("after") && !entry.contains('{')),
        "the record after the long entry was not formatted: {entries:?}"
    );
    assert_eq!(
        entries.iter().any(|entry| entry.contains("xxxxxxxx")),
        shown,
        "{entries:?}"
    );
}
