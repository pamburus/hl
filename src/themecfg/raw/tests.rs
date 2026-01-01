use std::collections::HashMap;

use crate::level::Level;

use super::super::{
    Color, Element, Merge, MergeFlags, MergeOptions, MergeWithOptions, Mode, ModeSetDiff, PlainColor, RGB, RawStyle,
    RawTheme, Role, StyleBase, Version,
    tests::{dirs, modes, raw_theme},
    v1,
};

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

#[test]
fn test_style_merge() {
    let base = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Bold.into(),
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: Some(Color::Plain(PlainColor::Blue)),
    };

    let patch = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Italic.into(),
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: None,
    };

    let result = base.clone().merged(&patch, MergeFlags::default());

    assert_eq!(result.modes, ModeSetDiff::from(Mode::Bold | Mode::Italic));
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Green)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Blue)));

    let patch = RawStyle {
        background: Some(Color::Plain(PlainColor::Green)),
        ..Default::default()
    };

    let result = base.clone().merged(&patch, MergeFlags::default());

    assert_eq!(result.modes, ModeSetDiff::from(Mode::Bold));
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Green)));
}

#[test]
fn test_v0_unknown_elements_ignored() {
    let theme = raw_theme("v0-unknown-elements").resolve().unwrap();

    assert_eq!(theme.elements.len(), 1);
    assert!(theme.elements.contains_key(&Element::Message));
}

#[test]
fn test_v0_unknown_level_names_ignored() {
    let theme = raw_theme("v0-unknown-levels");

    assert!(theme.levels.contains_key(&Level::Error), "Should have error level");

    assert_eq!(
        theme.levels.len(),
        1,
        "Should have only 1 valid level (unknown levels dropped during v0->v1 conversion)"
    );
}

#[test]
fn test_v0_style_merged_modes() {
    let base = RawStyle {
        base: StyleBase::default(),
        modes: super::super::tests::modes(&[Mode::Bold, Mode::Italic]),
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: None,
    };

    let patch_with_modes = RawStyle {
        base: StyleBase::default(),
        modes: (Mode::Underline).into(),
        foreground: None,
        background: Some(Color::Plain(PlainColor::Blue)),
    };

    let result = base.clone().merged(&patch_with_modes, Version::V0.merge_options());
    assert_eq!(result.modes, Mode::Underline.into());

    let patch_empty_modes = RawStyle {
        base: StyleBase::default(),
        modes: Default::default(),
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: None,
    };

    let result = base.clone().merged(&patch_empty_modes, Version::V0.merge_options());
    assert_eq!(result.modes, Default::default());
}

#[test]
fn test_v1_multiple_inheritance() {
    let theme = raw_theme("v1-multiple-inheritance");

    assert_eq!(theme.version, Version::V1_0);

    let flags = theme.merge_options();
    let inventory = theme.styles.resolved(flags).unwrap();

    let warning = &inventory[&Role::Warning];
    assert_eq!(warning.foreground, Some(Color::RGB(RGB(0x88, 0x88, 0x88))));
    assert_eq!(warning.background, Some(Color::RGB(RGB(0x33, 0x11, 0x00))));
    assert!(warning.modes.adds.contains(Mode::Faint));
    assert!(warning.modes.adds.contains(Mode::Bold));
    assert!(warning.modes.adds.contains(Mode::Underline));

    let error = &inventory[&Role::Error];
    assert_eq!(error.foreground, Some(Color::RGB(RGB(0xff, 0x00, 0x00))));
    assert_eq!(error.background, Some(Color::RGB(RGB(0x33, 0x11, 0x00))));
    assert!(error.modes.adds.contains(Mode::Faint));

    let theme = theme.resolve().unwrap();
    let level = &theme.elements[&Element::Level];
    assert_eq!(level.foreground, Some(Color::RGB(RGB(0x88, 0x88, 0x88))));
    assert!(level.modes.adds.contains(Mode::Faint));
    assert!(level.modes.adds.contains(Mode::Bold));

    let inner = &theme.elements[&Element::LevelInner];
    assert_eq!(inner.foreground, Some(Color::RGB(RGB(0x00, 0xff, 0x00))));
    assert!(inner.modes.adds.contains(Mode::Faint));
    assert!(inner.modes.adds.contains(Mode::Bold));
    assert!(inner.modes.adds.contains(Mode::Italic));
}

#[test]
fn test_v1_style_recursion_limit_error() {
    let app_dirs = dirs();

    let result = super::super::Theme::load(&app_dirs, "v1-recursion-circular");
    assert!(result.is_err());

    let err = result.unwrap_err();

    let err_msg = err.to_string();
    assert!(err_msg.contains("v1-recursion-circular"));
    assert!(err_msg.contains("style inheritance depth exceeded limit"));
    assert!(err_msg.contains("role"));

    match err {
        super::super::Error::FailedToResolveTheme { info, source } => {
            assert_eq!(info.name.as_ref(), "v1-recursion-circular");

            match source {
                super::super::StyleResolveError::RecursionLimitExceeded { role, .. } => {
                    assert!(
                        role == Role::Primary || role == Role::Secondary,
                        "Expected recursion in Primary or Secondary, got: {:?}",
                        role
                    );
                }
            }
        }
        other => panic!(
            "Expected FailedToResolveTheme wrapping StyleRecursionLimitExceeded, got: {:?}",
            other
        ),
    }
}

#[test]
fn test_v1_element_replacement_removes_parent_modes() {
    let mut parent_elements: HashMap<Element, RawStyle> = HashMap::new();
    parent_elements.insert(
        Element::Caller,
        RawStyle::new().base(Role::Secondary).modes(Mode::Italic),
    );

    let mut child_elements: HashMap<Element, RawStyle> = HashMap::new();
    child_elements.insert(Element::Caller, RawStyle::new().base(Role::Secondary));

    parent_elements.extend(child_elements);

    let result = &parent_elements[&Element::Caller];

    assert!(
        result.modes.is_empty(),
        "Child element should completely replace parent's element, not inherit modes"
    );

    assert!(!result.base.is_empty(), "Child element should have its own base");
}

#[test]
fn test_empty_v0_theme_file_valid() {
    let theme = raw_theme("v0-empty");

    assert_eq!(theme.version, Version::V0_0, "Empty file should be treated as v0 theme");

    assert_eq!(
        theme.elements.len(),
        0,
        "Empty v0 theme should have no elements defined"
    );
    assert_eq!(
        theme.styles.len(),
        0,
        "Empty v0 theme should have no styles (v0 doesn't support styles)"
    );
    assert_eq!(theme.tags.len(), 0, "Empty v0 theme should have no tags");
}

#[test]
fn test_v0_ignores_styles_section() {
    let theme = raw_theme("v0-with-styles-section");

    assert_eq!(theme.version, Version::V0_0, "Theme without version should be v0");

    let message = theme.elements.get(&Element::Message);
    assert!(message.is_some(), "Message element should be loaded");
    assert_eq!(
        message.unwrap().foreground,
        Some(Color::Plain(PlainColor::Green)),
        "Message should have green foreground from elements section"
    );

    assert!(
        !theme.styles.contains_key(&Role::Primary),
        "V0 theme should not have 'primary' style from file (styles section should be ignored)"
    );
    assert!(
        !theme.styles.contains_key(&Role::Secondary),
        "V0 theme should not have 'secondary' style from file (styles section should be ignored)"
    );

    let strong_style = theme.styles.get(&Role::Strong);
    assert!(
        strong_style.is_some(),
        "V0 theme should have 'strong' style deduced from message element"
    );
    assert_eq!(
        strong_style.unwrap().foreground,
        Some(Color::Plain(PlainColor::Green)),
        "Deduced 'strong' style should match message element foreground"
    );
}

#[test]
fn test_v0_parent_inner_blocking_all_pairs() {
    let mut base = RawTheme::default();

    base.elements.insert(Element::LevelInner, RawStyle::default());
    base.elements.insert(Element::LoggerInner, RawStyle::default());
    base.elements.insert(Element::CallerInner, RawStyle::default());
    base.elements.insert(Element::InputNumberInner, RawStyle::default());
    base.elements.insert(Element::InputNameInner, RawStyle::default());

    let mut child = RawTheme::default();
    child.elements.insert(Element::Level, RawStyle::default());
    child.elements.insert(Element::Logger, RawStyle::default());
    child.elements.insert(Element::Caller, RawStyle::default());
    child.elements.insert(Element::InputNumber, RawStyle::default());
    child.elements.insert(Element::InputName, RawStyle::default());

    let merged = base.merged(child);

    assert!(
        !merged.elements.contains_key(&Element::LevelInner),
        "level-inner should be blocked"
    );
    assert!(
        !merged.elements.contains_key(&Element::LoggerInner),
        "logger-inner should be blocked"
    );
    assert!(
        !merged.elements.contains_key(&Element::CallerInner),
        "caller-inner should be blocked"
    );
    assert!(
        !merged.elements.contains_key(&Element::InputNumberInner),
        "input-number-inner should be blocked"
    );
    assert!(
        !merged.elements.contains_key(&Element::InputNameInner),
        "input-name-inner should be blocked"
    );

    assert!(merged.elements.contains_key(&Element::Level), "level should be present");
    assert!(
        merged.elements.contains_key(&Element::Logger),
        "logger should be present"
    );
    assert!(
        merged.elements.contains_key(&Element::Caller),
        "caller should be present"
    );
    assert!(
        merged.elements.contains_key(&Element::InputNumber),
        "input-number should be present"
    );
    assert!(
        merged.elements.contains_key(&Element::InputName),
        "input-name should be present"
    );
}

#[test]
fn test_v0_level_section_blocking() {
    let mut base = RawTheme::default();

    let mut error_pack = v1::StylePack::default();
    error_pack.insert(
        Element::Message,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Red)),
            background: None,
            modes: Default::default(),
        },
    );
    error_pack.insert(
        Element::Level,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Blue)),
            background: None,
            modes: Default::default(),
        },
    );
    base.levels.insert(Level::Error, error_pack);

    let mut info_pack = v1::StylePack::default();
    info_pack.insert(
        Element::Message,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Green)),
            background: None,
            modes: Default::default(),
        },
    );
    base.levels.insert(Level::Info, info_pack);

    let mut child = RawTheme::default();
    let mut child_error_pack = v1::StylePack::default();
    child_error_pack.insert(
        Element::Time,
        RawStyle {
            base: StyleBase::default(),
            modes: modes(&[Mode::Bold]),
            foreground: None,
            background: None,
        },
    );
    child.levels.insert(Level::Error, child_error_pack);

    let merged = base.merged(child);

    let error_level = &merged.levels[&Level::Error];
    assert!(error_level.contains_key(&Element::Time), "Child time should be present");
    assert!(
        !error_level.contains_key(&Element::Message),
        "Base error message should be blocked"
    );
    assert!(
        !error_level.contains_key(&Element::Level),
        "Base error level should be blocked"
    );

    let info_level = &merged.levels[&Level::Info];
    assert!(
        info_level.contains_key(&Element::Message),
        "Base info message should remain"
    );
}

#[test]
fn test_v0_multiple_blocking_rules_combined() {
    let mut base = RawTheme::default();

    base.elements.insert(Element::LevelInner, RawStyle::default());
    base.elements.insert(Element::LoggerInner, RawStyle::default());

    base.elements.insert(Element::InputNumber, RawStyle::default());
    base.elements.insert(Element::InputName, RawStyle::default());

    let mut error_pack = v1::StylePack::default();
    error_pack.insert(Element::Message, RawStyle::default());
    base.levels.insert(Level::Error, error_pack);

    let mut child = RawTheme::default();
    child.elements.insert(Element::Level, RawStyle::default());
    child.elements.insert(Element::Logger, RawStyle::default());
    child.elements.insert(Element::Input, RawStyle::default());

    let mut child_error_pack = v1::StylePack::default();
    child_error_pack.insert(Element::Time, RawStyle::default());
    child.levels.insert(Level::Error, child_error_pack);

    let merged = base.merged(child);

    assert!(
        !merged.elements.contains_key(&Element::LevelInner),
        "level-inner blocked by parent rule"
    );
    assert!(
        !merged.elements.contains_key(&Element::LoggerInner),
        "logger-inner blocked by parent rule"
    );
    assert!(
        !merged.elements.contains_key(&Element::InputNumber),
        "input-number blocked by input rule"
    );
    assert!(
        !merged.elements.contains_key(&Element::InputName),
        "input-name blocked by input rule"
    );

    let error_level = &merged.levels[&Level::Error];
    assert!(
        !error_level.contains_key(&Element::Message),
        "Base error message blocked by level section rule"
    );
    assert!(
        error_level.contains_key(&Element::Time),
        "Child error time should be present"
    );
}

#[test]
fn test_v1_no_blocking_rules() {
    let mut base = RawTheme::default();
    base.inner_mut().version = Version { major: 1, minor: 0 };

    base.elements.insert(
        Element::LevelInner,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Red)),
            background: None,
            modes: Default::default(),
        },
    );

    base.elements.insert(Element::InputNumber, RawStyle::default());
    base.elements.insert(Element::InputName, RawStyle::default());

    let mut error_pack = v1::StylePack::default();
    error_pack.insert(
        Element::Message,
        RawStyle {
            base: StyleBase::default(),
            foreground: Some(Color::Plain(PlainColor::Red)),
            background: None,
            modes: Default::default(),
        },
    );
    base.levels.insert(Level::Error, error_pack);

    let mut child = RawTheme::default();
    child.inner_mut().version = Version { major: 1, minor: 0 };
    child.elements.insert(Element::Level, RawStyle::default());
    child.elements.insert(Element::Input, RawStyle::default());

    let mut child_error_pack = v1::StylePack::default();
    child_error_pack.insert(Element::Time, RawStyle::default());
    child.levels.insert(Level::Error, child_error_pack);

    let merged = base.merged(child);

    assert!(
        merged.elements.contains_key(&Element::LevelInner),
        "v1 should NOT block level-inner"
    );
    assert!(
        merged.elements.contains_key(&Element::InputNumber),
        "v1 should NOT block input-number"
    );
    assert!(
        merged.elements.contains_key(&Element::Level),
        "Child level should be present"
    );
    assert!(
        merged.elements.contains_key(&Element::Input),
        "Child input should be present"
    );

    let error_level = &merged.levels[&Level::Error];
    assert!(
        error_level.contains_key(&Element::Message),
        "v1 should preserve base error message"
    );
    assert!(
        error_level.contains_key(&Element::Time),
        "v1 should have child error time"
    );
}

#[test]
fn test_style_from_role() {
    let style = RawStyle::from(Role::Primary);
    assert!(!style.base.is_empty());
    assert_eq!(style.base.len(), 1);
    assert_eq!(style.base[0], Role::Primary);
}

#[test]
fn test_style_from_vec_roles() {
    let style = RawStyle::from(vec![Role::Primary, Role::Secondary]);
    assert!(!style.base.is_empty());
    assert_eq!(style.base.len(), 2);
    assert_eq!(style.base[0], Role::Primary);
    assert_eq!(style.base[1], Role::Secondary);
}

#[test]
fn test_resolved_style_builder_methods() {
    let style = RawStyle::default()
        .modes(Mode::Bold)
        .foreground(Some(Color::Plain(PlainColor::Red)))
        .background(Some(Color::Plain(PlainColor::Blue)));

    assert_eq!(style.modes, Mode::Bold.into());
    assert_eq!(style.foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(style.background, Some(Color::Plain(PlainColor::Blue)));
}

#[test]
fn test_resolved_style_merged_style_additive() {
    let base = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Bold.into(),
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: None,
    };

    let patch = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Italic.into(),
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: Some(Color::Plain(PlainColor::Blue)),
    };

    let merged = base.merged(&patch, MergeFlags::default());
    assert_eq!(merged.modes, ModeSetDiff::from(Mode::Bold | Mode::Italic));
    assert_eq!(merged.foreground, Some(Color::Plain(PlainColor::Green)));
    assert_eq!(merged.background, Some(Color::Plain(PlainColor::Blue)));
}

#[test]
fn test_resolved_style_merged_style_replace_modes() {
    let base = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Bold.into(),
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: None,
    };

    let patch = RawStyle {
        base: StyleBase::default(),
        modes: Mode::Italic.into(),
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: None,
    };

    let merged = base.merged(&patch, Version::V0.merge_options());
    assert_eq!(merged.modes, Mode::Italic.into());
    assert_eq!(merged.foreground, Some(Color::Plain(PlainColor::Green)));
}
