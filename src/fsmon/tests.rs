use super::*;

#[cfg(not(target_os = "macos"))]
use notify::{
    EventKind,
    event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode},
};
use rstest::rstest;

#[cfg(not(target_os = "macos"))]
#[rstest]
#[case::access(EventKind::Access(AccessKind::Any), None)]
#[case::close_write(EventKind::Access(AccessKind::Close(AccessMode::Write)), None)]
#[case::create(EventKind::Create(CreateKind::File), Some(Change::Modified))]
#[case::remove(EventKind::Remove(RemoveKind::File), Some(Change::Removed))]
#[case::modify_data(EventKind::Modify(ModifyKind::Data(DataChange::Any)), Some(Change::Modified))]
#[case::rename(EventKind::Modify(ModifyKind::Name(RenameMode::Any)), Some(Change::Modified))]
#[case::any(EventKind::Any, Some(Change::Modified))]
#[case::other(EventKind::Other, Some(Change::Modified))]
fn test_change_from_kind(#[case] kind: EventKind, #[case] change: Option<Change>) {
    assert_eq!(Change::from_kind(&kind), change);
}

#[rstest]
#[case::watched(&["/d/a.log"], true)]
#[case::other(&["/d/c.log"], false)]
#[case::directory(&["/d"], false)]
#[case::rename_to_watched(&["/d/c.log", "/d/b.log"], true)]
#[case::no_paths(&[], false)]
fn test_concerns(#[case] changed: &[&str], #[case] expected: bool) {
    let changed: Vec<PathBuf> = changed.iter().map(PathBuf::from).collect();
    let paths = [PathBuf::from("/d/a.log"), PathBuf::from("/d/b.log")];
    assert_eq!(concerns(&changed, &paths), expected);
}
