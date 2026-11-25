//! Input files.

use std::time::Duration;

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
            hl.wait_line(&format!("record-of-{name}"), Duration::from_secs(10)),
            "the record of {name}.log was not shown"
        );
    }

    let status = hl.stop(|| {}).expect("hl did not exit");
    assert!(status.success(), "hl exited with {status}");
}
