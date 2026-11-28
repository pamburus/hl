use std::{fs::OpenOptions, io::Write, path::Path};

/// Appends `data` to the file at `path` in one write, creating the file if it does not exist.
pub fn append(path: &Path, data: impl AsRef<[u8]>) {
    let mut file = OpenOptions::new().create(true).append(true).open(path).unwrap();
    file.write_all(data.as_ref()).unwrap();
}

/// Appends a JSON record with message `msg` to the file at `path`, creating the file if it does not exist.
pub fn append_record(path: &Path, msg: &str) {
    append(path, format!("{}\n", json::json!({"level": "info", "msg": msg})));
}

/// Returns a JSON record with timestamp `ts` and message `msg`, ended by a line break.
pub fn record(ts: &str, msg: &str) -> String {
    format!("{}\n", json::json!({"ts": ts, "level": "info", "msg": msg}))
}
