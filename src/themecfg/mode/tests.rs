use super::super::tests::raw_theme;
use super::super::{Mode, ModeDiff, ModeSet, ModeSetDiff};
use crate::themecfg::Element;

#[test]
fn test_mode_set_diff_with_removes() {
    let theme = raw_theme("test-mode-diff");
    let message = &theme.elements[&Element::Message];
    assert!(message.modes.adds.contains(Mode::Bold));
    assert!(message.modes.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_serialization() {
    let mut diff = ModeSetDiff::default();
    diff.adds.insert(Mode::Bold);
    diff.adds.insert(Mode::Italic);
    diff.removes.insert(Mode::Underline);

    let json = json::to_string(&diff).unwrap();
    assert!(json.contains("bold") || json.contains("Bold"));
    assert!(json.contains("italic") || json.contains("Italic"));
    assert!(json.contains("underline") || json.contains("Underline"));
}

#[test]
fn test_mode_diff_serialization() {
    let add_diff = ModeDiff::add(Mode::Bold);
    let json = json::to_string(&add_diff).unwrap();
    assert!(json.contains("+bold") || json.contains("bold"));

    let remove_diff = ModeDiff::remove(Mode::Italic);
    let json = json::to_string(&remove_diff).unwrap();
    assert!(json.contains("-italic") || json.contains("italic"));
}

#[test]
fn test_mode_set_add_mode_set_diff() {
    let mut set = ModeSet::new();
    set.insert(Mode::Bold);

    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Italic);
    diff.removes.insert(Mode::Bold);

    let result = set + diff;
    assert!(!result.contains(Mode::Bold));
    assert!(result.contains(Mode::Italic));
}

#[test]
fn test_mode_set_add_assign_mode_set_diff() {
    let mut set = ModeSet::new();
    set.insert(Mode::Bold);

    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Italic);
    diff.removes.insert(Mode::Bold);

    set += diff;
    assert!(!set.contains(Mode::Bold));
    assert!(set.contains(Mode::Italic));
}

#[test]
fn test_mode_set_sub_mode_set_diff() {
    let mut set = ModeSet::new();
    set.insert(Mode::Bold);

    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Bold);
    diff.removes.insert(Mode::Italic);

    let result = set - diff;
    assert!(!result.contains(Mode::Bold));
    assert!(result.contains(Mode::Italic));
}

#[test]
fn test_mode_set_sub_assign_mode_set_diff() {
    let mut set = ModeSet::new();
    set.insert(Mode::Bold);

    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Bold);
    diff.removes.insert(Mode::Italic);

    set -= diff;
    assert!(!set.contains(Mode::Bold));
    assert!(set.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_reversed() {
    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Bold);
    diff.removes.insert(Mode::Italic);

    let reversed = diff.reversed();
    assert!(reversed.adds.contains(Mode::Italic));
    assert!(reversed.removes.contains(Mode::Bold));
}

#[test]
fn test_mode_set_diff_neg() {
    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Bold);
    diff.removes.insert(Mode::Italic);

    let negated = -diff;
    assert!(negated.adds.contains(Mode::Italic));
    assert!(negated.removes.contains(Mode::Bold));
}

#[test]
fn test_mode_set_diff_add_mode_set_diff() {
    let mut diff1 = ModeSetDiff::new();
    diff1.adds.insert(Mode::Bold);
    diff1.removes.insert(Mode::Italic);

    let mut diff2 = ModeSetDiff::new();
    diff2.adds.insert(Mode::Underline);
    diff2.removes.insert(Mode::Bold);

    let result = diff1 + diff2;
    assert!(!result.adds.contains(Mode::Bold));
    assert!(result.adds.contains(Mode::Underline));
    assert!(result.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_add_assign_mode_set_diff() {
    let mut diff1 = ModeSetDiff::new();
    diff1.adds.insert(Mode::Bold);
    diff1.removes.insert(Mode::Italic);

    let mut diff2 = ModeSetDiff::new();
    diff2.adds.insert(Mode::Underline);
    diff2.removes.insert(Mode::Bold);

    diff1 += diff2;
    assert!(!diff1.adds.contains(Mode::Bold));
    assert!(diff1.adds.contains(Mode::Underline));
    assert!(diff1.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_sub_mode_set_diff() {
    let mut diff1 = ModeSetDiff::new();
    diff1.adds.insert(Mode::Bold);
    diff1.removes.insert(Mode::Italic);

    let mut diff2 = ModeSetDiff::new();
    diff2.adds.insert(Mode::Bold);
    diff2.removes.insert(Mode::Underline);

    let result = diff1 - diff2;
    assert!(!result.adds.contains(Mode::Bold));
    assert!(result.adds.contains(Mode::Underline));
    assert!(result.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_sub_assign_mode_set_diff() {
    let mut diff1 = ModeSetDiff::new();
    diff1.adds.insert(Mode::Bold);
    diff1.removes.insert(Mode::Italic);

    let mut diff2 = ModeSetDiff::new();
    diff2.adds.insert(Mode::Bold);
    diff2.removes.insert(Mode::Underline);

    diff1 -= diff2;
    assert!(!diff1.adds.contains(Mode::Bold));
    assert!(diff1.adds.contains(Mode::Underline));
    assert!(diff1.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_add_mode_set() {
    let mut diff = ModeSetDiff::new();
    diff.removes.insert(Mode::Italic);

    let mut set = ModeSet::new();
    set.insert(Mode::Bold);

    let result = diff + set;
    assert!(result.adds.contains(Mode::Bold));
    assert!(result.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_add_assign_mode_set() {
    let mut diff = ModeSetDiff::new();
    diff.removes.insert(Mode::Italic);

    let mut set = ModeSet::new();
    set.insert(Mode::Bold);

    diff += set;
    assert!(diff.adds.contains(Mode::Bold));
    assert!(diff.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_sub_mode_set() {
    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Bold);

    let mut set = ModeSet::new();
    set.insert(Mode::Italic);

    let result = diff - set;
    assert!(result.adds.contains(Mode::Bold));
    assert!(result.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_sub_assign_mode_set() {
    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Bold);

    let mut set = ModeSet::new();
    set.insert(Mode::Italic);

    diff -= set;
    assert!(diff.adds.contains(Mode::Bold));
    assert!(diff.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_add_mode() {
    let mut diff = ModeSetDiff::new();
    diff.removes.insert(Mode::Italic);

    let result = diff + Mode::Bold;
    assert!(result.adds.contains(Mode::Bold));
    assert!(result.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_add_assign_mode() {
    let mut diff = ModeSetDiff::new();
    diff.removes.insert(Mode::Italic);

    diff += Mode::Bold;
    assert!(diff.adds.contains(Mode::Bold));
    assert!(diff.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_sub_mode() {
    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Bold);

    let result = diff - Mode::Italic;
    assert!(result.adds.contains(Mode::Bold));
    assert!(result.removes.contains(Mode::Italic));
}

#[test]
fn test_mode_set_diff_sub_assign_mode() {
    let mut diff = ModeSetDiff::new();
    diff.adds.insert(Mode::Bold);

    diff -= Mode::Italic;
    assert!(diff.adds.contains(Mode::Bold));
    assert!(diff.removes.contains(Mode::Italic));
}
