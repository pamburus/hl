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
    assert_eq!(StylePack::<Element, ElementStyle>::default().clone().len(), 0);

    let yaml = include_str!("../testing/assets/style-packs/pack1.yaml");
    let pack: StylePack<Element> = yaml::from_str(yaml).unwrap().remove(0);
    assert_eq!(pack.0.len(), 2);
    assert_eq!(
        pack.0[&Element::Input].patch.foreground,
        Some(Color::Plain(PlainColor::Red))
    );
    assert_eq!(
        pack.0[&Element::Input].patch.background,
        Some(Color::Plain(PlainColor::Blue))
    );
    assert_eq!(pack.0[&Element::Input].patch.modes, vec![Mode::Bold, Mode::Faint]);
    assert_eq!(
        pack.0[&Element::Message].patch.foreground,
        Some(Color::Plain(PlainColor::Green))
    );
    assert_eq!(pack.0[&Element::Message].patch.background, None);
    assert_eq!(
        pack.0[&Element::Message].patch.modes,
        vec![Mode::Italic, Mode::Underline]
    );

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
    let base = Style {
        modes: vec![Mode::Bold],
        foreground: Some(Color::Plain(PlainColor::Red)),
        background: Some(Color::Plain(PlainColor::Blue)),
    };

    let patch = Style {
        modes: vec![Mode::Italic],
        foreground: Some(Color::Plain(PlainColor::Green)),
        background: None,
    };

    let result = base.clone().merged(&patch);

    assert_eq!(result.modes, vec![Mode::Italic]);
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Green)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Blue)));

    let patch = Style {
        modes: vec![],
        foreground: None,
        background: Some(Color::Plain(PlainColor::Green)),
    };

    let result = base.clone().merged(&patch);

    assert_eq!(result.modes, vec![Mode::Bold]);
    assert_eq!(result.foreground, Some(Color::Plain(PlainColor::Red)));
    assert_eq!(result.background, Some(Color::Plain(PlainColor::Green)));
}
