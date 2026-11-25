use std::{fs::OpenOptions, io::Write, path::Path};

/// Appends a JSON record with message `msg` to the file at `path`, which must exist.
pub fn append_record(path: &Path, msg: &str) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    writeln!(file, "{}", json::json!({"level": "info", "msg": msg})).unwrap();
}
