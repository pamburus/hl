use super::*;

#[test]
fn test_load() {
    let app_dirs = AppDirs {
        config_dir: PathBuf::from("src/testing/assets"),
        cache_dir: Default::default(),
        system_config_dirs: Default::default(),
    };
    assert_ne!(Theme::load(&app_dirs, "test").unwrap().elements.len(), 0);
    assert_ne!(Theme::load(&app_dirs, "universal").unwrap().elements.len(), 0);
    assert!(Theme::load(&app_dirs, "non-existent").is_err());
    assert!(Theme::load(&app_dirs, "invalid").is_err());
    assert!(Theme::load(&app_dirs, "invalid-type").is_err());
}

#[test]
fn test_load_from() {
    let path = PathBuf::from("etc/defaults/themes");
    assert_ne!(Theme::load_from(&path, "universal").unwrap().elements.len(), 0);

    let path = PathBuf::from("src/testing/assets/themes");
    assert_ne!(Theme::load_from(&path, "test").unwrap().elements.len(), 0);
    assert_ne!(Theme::load_from(&path, "test.toml").unwrap().elements.len(), 0);
    assert_ne!(
        Theme::load_from(&path, "./src/testing/assets/themes/test.toml")
            .unwrap()
            .elements
            .len(),
        0
    );
    assert!(Theme::load_from(&path, "non-existent").is_err());
    assert!(Theme::load_from(&path, "invalid").is_err());
    assert!(Theme::load_from(&path, "invalid-type").is_err());
}

#[test]
fn test_embedded() {
    assert_ne!(Theme::embedded("universal").unwrap().elements.len(), 0);
    assert!(Theme::embedded("non-existent").is_err());
}

#[test]
fn test_rgb() {
    let a = RGB::from_str("#102030").unwrap();
    assert_eq!(a, RGB(16, 32, 48));
    let b: RGB = serde_json::from_str(r##""#102030""##).unwrap();
    assert_eq!(b, RGB(16, 32, 48));
}

#[test]
fn test_style_pack() {
    assert_eq!(StylePack::<Element>::default().clone().len(), 0);

    let yaml = include_str!("../testing/assets/style-packs/pack1.yaml");
    let pack: StylePack<Element> = yaml::from_str(yaml).unwrap().remove(0);
    assert_eq!(pack.0.len(), 2);
    assert_eq!(pack.0[&Element::Input].foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(pack.0[&Element::Input].background, Some(Color::Plain(PlainColor::Blue)));
    assert_eq!(pack.0[&Element::Input].modes, (Mode::Bold | Mode::Faint).into());
    assert_eq!(
        pack.0[&Element::Message].foreground,
        Some(Color::Plain(PlainColor::Green))
    );
    assert_eq!(pack.0[&Element::Message].background, None);
    assert_eq!(pack.0[&Element::Message].modes, (Mode::Italic | Mode::Underline).into());

    assert!(
        yaml::from_str::<StylePack<Element>>("invalid")
            .unwrap_err()
            .msg
            .ends_with("expected style pack object")
    );
}

#[test]
fn test_tags() {
    assert_eq!(Tag::from_str("dark").unwrap(), Tag::Dark);
    assert_eq!(Tag::from_str("light").unwrap(), Tag::Light);
    assert_eq!(Tag::from_str("16color").unwrap(), Tag::Palette16);
    assert_eq!(Tag::from_str("256color").unwrap(), Tag::Palette256);
    assert_eq!(Tag::from_str("truecolor").unwrap(), Tag::TrueColor);
    assert!(Tag::from_str("invalid").is_err());
}

#[test]
fn test_style_merge() {
    let base = ResolvedStyle {
        modes: Mode::Bold.into(),
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: Some(Color::Plain(PlainColor::Blue)),
    };

    let patch = ResolvedStyle {
        modes: Mode::Italic.into(),
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: None,
    };

    let result = base.clone().merged_with(&patch, MergeFlags::default());

    assert_eq!(result.modes, ModeSetDiff::from(Mode::Bold | Mode::Italic));
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Green)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Blue)));

    let patch = ResolvedStyle {
        background: Some(Color::Plain(PlainColor::Green)),
        ..Default::default()
    };

    let result = base.clone().merged_with(&patch, MergeFlags::default());

    assert_eq!(result.modes, ModeSetDiff::from(Mode::Bold));
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Green)));
}

#[test]
fn test_theme_version_parsing() {
    // Valid versions
    assert_eq!(ThemeVersion::from_str("1.0").unwrap(), ThemeVersion::new(1, 0));
    assert_eq!(ThemeVersion::from_str("1.10").unwrap(), ThemeVersion::new(1, 10));
    assert_eq!(ThemeVersion::from_str("2.123").unwrap(), ThemeVersion::new(2, 123));
    assert_eq!(ThemeVersion::from_str("0.0").unwrap(), ThemeVersion::new(0, 0));

    // Invalid versions - leading zeros
    assert!(ThemeVersion::from_str("1.01").is_err());
    assert!(ThemeVersion::from_str("01.0").is_err());
    assert!(ThemeVersion::from_str("01.01").is_err());

    // Invalid versions - missing components
    assert!(ThemeVersion::from_str("1").is_err());
    assert!(ThemeVersion::from_str("1.").is_err());
    assert!(ThemeVersion::from_str(".1").is_err());

    // Invalid versions - not numbers
    assert!(ThemeVersion::from_str("1.x").is_err());
    assert!(ThemeVersion::from_str("x.1").is_err());
    assert!(ThemeVersion::from_str("a.b").is_err());

    // Invalid versions - extra components
    assert!(ThemeVersion::from_str("1.0.0").is_err());
}

#[test]
fn test_theme_version_display() {
    assert_eq!(ThemeVersion::new(1, 0).to_string(), "1.0");
    assert_eq!(ThemeVersion::new(1, 10).to_string(), "1.10");
    assert_eq!(ThemeVersion::new(2, 123).to_string(), "2.123");
    assert_eq!(ThemeVersion::new(0, 0).to_string(), "0.0");
}

#[test]
fn test_theme_version_compatibility() {
    let v1_0 = ThemeVersion::new(1, 0);
    let v1_1 = ThemeVersion::new(1, 1);
    let v1_2 = ThemeVersion::new(1, 2);
    let v2_0 = ThemeVersion::new(2, 0);

    // Same version is compatible
    assert!(v1_0.is_compatible_with(&v1_0));
    assert!(v1_1.is_compatible_with(&v1_1));

    // Older minor version is compatible
    assert!(v1_0.is_compatible_with(&v1_1));
    assert!(v1_0.is_compatible_with(&v1_2));
    assert!(v1_1.is_compatible_with(&v1_2));

    // Newer minor version is not compatible
    assert!(!v1_1.is_compatible_with(&v1_0));
    assert!(!v1_2.is_compatible_with(&v1_0));
    assert!(!v1_2.is_compatible_with(&v1_1));

    // Different major version is not compatible
    assert!(!v2_0.is_compatible_with(&v1_0));
    assert!(!v1_0.is_compatible_with(&v2_0));
}

#[test]
fn test_theme_version_serde() {
    // Deserialize
    let version: ThemeVersion = serde_json::from_str(r#""1.0""#).unwrap();
    assert_eq!(version, ThemeVersion::new(1, 0));

    let version: ThemeVersion = serde_json::from_str(r#""2.15""#).unwrap();
    assert_eq!(version, ThemeVersion::new(2, 15));

    // Serialize
    let version = ThemeVersion::new(1, 0);
    let json = serde_json::to_string(&version).unwrap();
    assert_eq!(json, r#""1.0""#);

    let version = ThemeVersion::new(2, 15);
    let json = serde_json::to_string(&version).unwrap();
    assert_eq!(json, r#""2.15""#);

    // Invalid formats should fail
    assert!(serde_json::from_str::<ThemeVersion>(r#""1.01""#).is_err());
    assert!(serde_json::from_str::<ThemeVersion>(r#""1""#).is_err());
    assert!(serde_json::from_str::<ThemeVersion>(r#"1"#).is_err());
}

#[test]
fn test_theme_version_constants() {
    assert_eq!(ThemeVersion::V0_0, ThemeVersion::new(0, 0));
    assert_eq!(ThemeVersion::V1_0, ThemeVersion::new(1, 0));
    assert_eq!(ThemeVersion::CURRENT, ThemeVersion::V1_0);
    assert_eq!(THEME_VERSION, ThemeVersion::CURRENT);
}
