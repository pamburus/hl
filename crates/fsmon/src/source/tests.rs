use rstest::rstest;

use super::*;

// ---

/// FR-005: only a path that is gone or holds another file counts as replaced, as a check that fails for another
/// reason may fail only briefly, and a replacement would read the same file again from its start. On Windows, where a
/// deleted file may stay at its path while it is open and cannot be opened there, a denied access counts as replaced.
#[rstest]
#[case::same(Ok(false), false)]
#[case::other(Ok(true), true)]
#[case::missing(Err(io::ErrorKind::NotFound), true)]
#[case::not_a_directory(Err(io::ErrorKind::NotADirectory), true)]
#[case::denied(Err(io::ErrorKind::PermissionDenied), cfg!(windows))]
#[case::failed(Err(io::ErrorKind::Other), false)]
fn test_replaced_only_when_another_file_or_none(#[case] found: Result<bool, io::ErrorKind>, #[case] expected: bool) {
    let (id, other) = (FileId::fake(1), FileId::fake(2));
    let found = found
        .map(|replaced| if replaced { other } else { id })
        .map_err(io::Error::from);
    assert_eq!(replaced(found, id), expected);
}
