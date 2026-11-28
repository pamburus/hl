use super::*;

// ---

/// A bare name is watched through the current directory, as a path in any other directory is through that one, so that
/// it is watched as natively as the same path given another way (FR-010).
#[rstest::rstest]
#[case::bare("app.log", ".")]
#[case::relative("logs/app.log", "logs")]
#[case::absolute("/var/log/app.log", "/var/log")]
fn test_dir_of_path(#[case] path: &str, #[case] dir: &str) {
    assert_eq!(dir_of(Path::new(path)), Path::new(dir));
}
