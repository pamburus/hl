use super::super::{Color, MergeFlag, MergeFlags, MergeWithOptions, Mode, ModeSetDiff, PlainColor, RawStyle};
use super::Style;

#[test]
fn test_style_builder_methods() {
    let style = Style::default()
        .foreground(Some(Color::Plain(PlainColor::Green)))
        .background(Some(Color::Plain(PlainColor::Black)));

    assert_eq!(style.foreground, Some(Color::Plain(PlainColor::Green)));
    assert_eq!(style.background, Some(Color::Plain(PlainColor::Black)));
}

#[test]
fn test_style_modes_builder() {
    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Bold);
    let style = Style::new().modes(diff);
    assert!(style.modes.adds.contains(Mode::Bold));
}

#[test]
fn test_style_merge_replace_modes() {
    let mut style1 = Style::new();
    style1.modes.adds.insert(Mode::Bold);

    let mut style2 = Style::new();
    style2.modes.adds.insert(Mode::Italic);

    style1.merge(&style2, MergeFlag::ReplaceModes.into());
    assert!(!style1.modes.adds.contains(Mode::Bold));
    assert!(style1.modes.adds.contains(Mode::Italic));
}

#[test]
fn test_style_merge_raw_style() {
    let mut style = Style::new();
    style.modes.adds.insert(Mode::Bold);

    let mut raw = RawStyle::default();
    raw.modes.adds.insert(Mode::Italic);
    raw.foreground = Some(Color::Plain(PlainColor::Red));

    style.merge(&raw, MergeFlags::default());
    assert!(style.modes.adds.contains(Mode::Bold));
    assert!(style.modes.adds.contains(Mode::Italic));
    assert_eq!(style.foreground, Some(Color::Plain(PlainColor::Red)));
}

#[test]
fn test_style_merge_raw_style_replace_modes() {
    let mut style = Style::new();
    style.modes.adds.insert(Mode::Bold);

    let mut raw = RawStyle::default();
    raw.modes.adds.insert(Mode::Italic);
    raw.foreground = Some(Color::Plain(PlainColor::Red));
    raw.background = Some(Color::Plain(PlainColor::Blue));

    style.merge(&raw, MergeFlag::ReplaceModes.into());
    assert!(!style.modes.adds.contains(Mode::Bold));
    assert!(style.modes.adds.contains(Mode::Italic));
    assert_eq!(style.foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(style.background, Some(Color::Plain(PlainColor::Blue)));
}
