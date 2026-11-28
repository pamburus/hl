//! Queries that load sets of strings from files of the OS.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use hl::{Query, error::Error};
use rstest::rstest;
use tempfile::TempDir;

use crate::support::{hl::Hl, repo::repo_file};

/// The path of the query set file `name` in the repository.
fn set_file(name: &str) -> PathBuf {
    repo_file(&format!("src/testing/assets/query/{name}"))
}

/// A query of whether field `v` is `in` or `not in` the set loaded from `path`.
///
/// The path is quoted, so its separators and spaces are not taken as part of the query.
fn in_set_file(op: &str, path: &Path) -> String {
    let path = json::to_string(path.to_str().expect("the path is not valid UTF-8")).unwrap();
    format!("v {op} @{path}")
}

/// The values of field `v` in the log, of which `set-valid` holds `line1` to `line3`.
const VALUES: [&str; 5] = ["line", "line1", "line2", "line3", "line4"];

#[rstest]
#[case::in_set("in", &["line1", "line2", "line3"])]
#[case::not_in_set("not in", &["line", "line4"])]
fn test_in_set_file_valid(#[case] op: &str, #[case] expected: &[&str]) {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("test.log");
    let output = dir.path().join("output.log");
    let records = VALUES.map(|v| json::json!({"level": "info", "msg": format!("[{v}]"), "v": v}).to_string());
    std::fs::write(&log, records.join("\n") + "\n").unwrap();

    let mut hl = Hl::spawn([
        "--query".as_ref(),
        in_set_file(op, &set_file("set-valid")).as_ref(),
        "--output".as_ref(),
        output.as_os_str(),
        log.as_os_str(),
    ]);
    let status = hl.stop(|| {}).expect("hl did not exit");
    assert!(status.success(), "hl exited with {status}");

    let output = std::fs::read_to_string(&output).unwrap();
    let shown: Vec<_> = VALUES
        .into_iter()
        .filter(|v| output.contains(&format!("[{v}]")))
        .collect();
    assert_eq!(shown, expected, "output:\n{output}");
}

#[test]
fn test_in_set_file_invalid() {
    let path = set_file("set-invalid");
    let Err(err) = Query::parse(in_set_file("in", &path)) else {
        panic!("the query was parsed");
    };
    if let Error::FailedToLoadFile { path: failed, source } = &err {
        assert_eq!(Path::new(failed), path);
        assert!(
            matches!(&**source, Error::FailedToParseJsonLine { line: 2, source } if source.is_eof()),
            "{source:?}"
        );
    } else {
        panic!("unexpected error: {err:?}");
    }
}

#[test]
fn test_in_set_file_not_found() {
    let path = set_file("set-not-found");
    let Err(err) = Query::parse(in_set_file("in", &path)) else {
        panic!("the query was parsed");
    };
    if let Error::FailedToReadFile { path: failed, source } = &err {
        assert_eq!(Path::new(failed), path);
        assert_eq!(source.kind(), ErrorKind::NotFound);
    } else {
        panic!("unexpected error: {err:?}");
    }
}
