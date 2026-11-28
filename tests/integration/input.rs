//! Input files.

use std::time::Duration;

use hl::input::{InputHolder, InputPath, InputReference};
use tempfile::TempDir;

use crate::support::{hl::Hl, log::append_record};

#[test]
fn test_shows_records_of_all_files() {
    // Created before the process, so it is removed after the process has stopped.
    let dir = TempDir::new().unwrap();
    let names = ["a", "b"];
    let files = names.map(|name| {
        let file = dir.path().join(format!("{name}.log"));
        append_record(&file, &format!("record-of-{name}"));
        file
    });

    let mut hl = Hl::spawn(&files);
    for name in names {
        assert!(
            hl.wait_entry(&format!("record-of-{name}"), Duration::from_secs(10))
                .is_some(),
            "the record of {name}.log was not shown"
        );
    }

    let status = hl.stop(|| {}).expect("hl did not exit");
    assert!(status.success(), "hl exited with {status}");
}

/// A file input that is not held has metadata but no open file, and one that is held has both.
#[rstest::rstest]
#[case::not_held(false)]
#[case::held(true)]
fn test_input_holder_of_file_has_metadata_and_file_once_held(#[case] held: bool) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("app.log");
    append_record(&path, "record");
    let holder = if held {
        InputReference::File(InputPath::resolve(path).unwrap()).hold().unwrap()
    } else {
        InputHolder::new(InputReference::File(InputPath::ephemeral(path)), None)
    };

    assert!(holder.metadata().unwrap().is_some());
    assert_eq!(holder.file().is_some(), held);
    assert_eq!(holder.into_file().is_ok(), held);
}
