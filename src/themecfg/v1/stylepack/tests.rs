use yaml_peg::serde as yaml;

use super::super::super::{Color, Element, Merge, Mode, PlainColor, tests::modes};
use super::super::{GetMergeFlags, Style, Version};
use super::StylePack;

#[test]
fn test_style_pack() {
    assert_eq!(StylePack::<Element>::default().len(), 0);

    let yaml = include_str!("../../../testing/assets/style-packs/pack1.yaml");
    let pack: StylePack<Element> = yaml::from_str(yaml).unwrap().remove(0);
    assert_eq!(pack.len(), 2);
    assert_eq!(pack[&Element::Input].foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(pack[&Element::Input].background, Some(Color::Plain(PlainColor::Blue)));
    assert_eq!(pack[&Element::Input].modes, modes(&[Mode::Bold, Mode::Faint]));
    assert_eq!(
        pack[&Element::Message].foreground,
        Some(Color::Plain(PlainColor::Green))
    );
    assert_eq!(pack[&Element::Message].background, None);
    assert_eq!(pack[&Element::Message].modes, modes(&[Mode::Italic, Mode::Underline]));

    assert!(yaml::from_str::<StylePack<Element>>("invalid").is_err());
}

#[test]
fn test_v1_style_pack_merge() {
    let mut base = StylePack::default();
    base.insert(
        Element::Message,
        Style {
            foreground: Some(Color::Plain(PlainColor::Red)),
            background: Some(Color::Plain(PlainColor::Blue)),
            modes: Mode::Bold.into(),
            ..Default::default()
        },
    );

    let mut patch = StylePack::<Element>::default();
    patch.insert(
        Element::Message,
        Style {
            foreground: Some(Color::Plain(PlainColor::Green)),
            modes: Mode::Italic.into(),
            ..Default::default()
        },
    );
    patch.insert(
        Element::Level,
        Style {
            foreground: Some(Color::Plain(PlainColor::Yellow)),
            ..Default::default()
        },
    );

    let merged = base.merged(patch, Version::V0.merge_flags());

    assert_eq!(
        merged[&Element::Message].foreground,
        Some(Color::Plain(PlainColor::Green))
    );

    assert_eq!(
        merged[&Element::Level].foreground,
        Some(Color::Plain(PlainColor::Yellow))
    );
}

#[test]
fn test_child_blocking_parent_in_style_pack() {
    let mut base = StylePack::default();
    base.insert(Element::Level, Style::default());

    let mut patch = StylePack::default();
    patch.insert(Element::LevelInner, Style::default());

    let merged = base.merged(&patch, Version::V0.merge_flags());

    assert!(!merged.contains_key(&Element::Level));
    assert!(merged.contains_key(&Element::LevelInner));
}
