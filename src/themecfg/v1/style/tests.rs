use std::collections::HashMap;

use super::super::super::{Color, Merge, MergeFlags, MergeWithOptions, Mode, PlainColor, Role, Style as ResolvedStyle};
use super::super::{Style, StyleBase, StylePack, StyleResolver};

#[test]
fn test_style_pack_merged() {
    let mut items1 = HashMap::new();
    items1.insert(Role::Primary, Style::default());
    let pack1 = StylePack::<Role, Style>::new(items1);

    let mut items2 = HashMap::new();
    let style2 = Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        ..Style::default()
    };
    items2.insert(Role::Secondary, style2);
    let pack2 = StylePack::<Role, Style>::new(items2);

    let merged = pack1.merged(pack2);
    let mut resolver = StyleResolver::new(&merged, MergeFlags::default());
    assert!(resolver.resolve(&Role::Primary).is_ok());
    assert!(resolver.resolve(&Role::Secondary).is_ok());
}

#[test]
fn test_style_reverse_merge() {
    let mut style1 = Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        ..Style::default()
    };

    let mut style2 = Style {
        foreground: Some(Color::Plain(PlainColor::Blue)),
        ..Style::default()
    };
    style2.modes.adds.insert(Mode::Bold);

    style1.reverse_merge(style2, MergeFlags::default());
    assert_eq!(style1.foreground, Some(Color::Plain(PlainColor::Red)));
    assert!(style1.modes.adds.contains(Mode::Bold));
}

#[test]
fn test_style_resolve_base_with() {
    let bases = StyleBase::from(vec![Role::Primary]);
    let style = Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        ..Style::default()
    };

    let resolved = Style::resolve_base_with(&bases, &style, MergeFlags::default(), |_| {
        let mut rs = ResolvedStyle::new();
        rs.modes.adds.insert(Mode::Bold);
        rs
    });

    assert_eq!(resolved.foreground, Some(Color::Plain(PlainColor::Red)));
    assert!(resolved.modes.adds.contains(Mode::Bold));
}

#[test]
fn test_style_resolve_with() {
    let bases = StyleBase::from(vec![Role::Primary]);
    let style = Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        ..Style::default()
    };

    let resolved = Style::resolve_with(&bases, &style, MergeFlags::default(), |_| {
        let mut rs = ResolvedStyle::new();
        rs.modes.adds.insert(Mode::Bold);
        rs
    });

    assert_eq!(resolved.foreground, Some(Color::Plain(PlainColor::Red)));
    assert!(resolved.modes.adds.contains(Mode::Bold));
}

#[test]
fn test_style_merge_owned() {
    let mut style1 = Style {
        foreground: Some(Color::Plain(PlainColor::Red)),
        base: StyleBase::from(vec![Role::Primary]),
        ..Style::default()
    };
    style1.modes.adds.insert(Mode::Bold);

    let mut style2 = Style {
        foreground: Some(Color::Plain(PlainColor::Blue)),
        base: StyleBase::from(vec![Role::Secondary]),
        ..Style::default()
    };
    style2.modes.adds.insert(Mode::Italic);

    style1.merge(style2, MergeFlags::default());
    assert_eq!(style1.foreground, Some(Color::Plain(PlainColor::Blue)));
    assert!(style1.modes.adds.contains(Mode::Bold));
    assert!(style1.modes.adds.contains(Mode::Italic));
    assert_eq!(style1.base, StyleBase::from(vec![Role::Secondary]));
}
