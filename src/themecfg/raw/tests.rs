use super::super::{RawTheme, Version};

#[test]
fn test_raw_theme_inner_mut() {
    let mut theme = RawTheme::default();
    theme.inner_mut().version = Version::new(1, 0);
    assert_eq!(theme.inner().version, Version::new(1, 0));
}

#[test]
fn test_raw_theme_into_inner() {
    let mut theme = RawTheme::default();
    theme.inner_mut().version = Version::new(1, 0);
    let inner = theme.into_inner();
    assert_eq!(inner.version, Version::new(1, 0));
}
