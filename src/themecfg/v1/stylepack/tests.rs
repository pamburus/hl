use yaml_peg::serde as yaml;

use super::super::super::{Color, Element, Merge, Mode, PlainColor, RawStyle, StyleBase, StylePack, tests::modes, v1};
use super::super::{GetMergeFlags, Version};

#[test]
fn test_style_pack() {
    assert_eq!(StylePack::default().len(), 0);

    let yaml = include_str!("../../../testing/assets/style-packs/pack1.yaml");
    let pack: v1::StylePack<Element> = yaml::from_str(yaml).unwrap().remove(0);
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

    assert!(yaml::from_str::<v1::StylePack<Element>>("invalid").is_err());
}

#[test]
fn test_v1_style_pack_merge() {
    let mut base = v1::StylePack::default();
    base.insert(
        Element::Message,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Red)),
            background: Some(Color::Plain(PlainColor::Blue)),
            modes: Mode::Bold.into(),
        },
    );

    let mut patch = v1::StylePack::<Element>::default();
    patch.insert(
        Element::Message,
        v1::Style {
            foreground: Some(Color::Plain(PlainColor::Green)),
            modes: Mode::Italic.into(),
            ..Default::default()
        },
    );
    patch.insert(
        Element::Level,
        v1::Style {
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
