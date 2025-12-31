use std::str::FromStr;

use super::super::tests::theme;
use super::super::{Color, Element, PlainColor};
use super::RGB;

#[test]
fn test_rgb() {
    let a = RGB::from_str("#102030").unwrap();
    assert_eq!(a, RGB(16, 32, 48));
    let b: RGB = serde_json::from_str(r##""#102030""##).unwrap();
    assert_eq!(b, RGB(16, 32, 48));
}

#[test]
fn test_rgb_lowercase() {
    let a = RGB::from_str("#aabbcc").unwrap();
    assert_eq!(a, RGB(170, 187, 204));
    let b = RGB::from_str("#AABBCC").unwrap();
    assert_eq!(b, RGB(170, 187, 204));
}

#[test]
fn test_rgb_invalid() {
    assert!(RGB::from_str("ff0000").is_err());

    assert!(RGB::from_str("#fff").is_err());
    assert!(RGB::from_str("#fffffff").is_err());

    assert!(RGB::from_str("#gghhii").is_err());
    assert!(RGB::from_str("#zzzzzz").is_err());
}

#[test]
fn test_rgb_display() {
    let rgb = RGB(255, 128, 64);
    let s = format!("{}", rgb);
    assert_eq!(s, "#ff8040");
}

#[test]
fn test_rgb_from_str_invalid_length() {
    let result = RGB::from_str("#ff");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected 7 bytes"));
}

#[test]
fn test_rgb_from_str_missing_hash() {
    let result = RGB::from_str("ff80400");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected # sign"));
}

#[test]
fn test_v0_color_formats() {
    let theme = theme("v0-color-formats");

    assert_eq!(
        theme.elements[&Element::Message].foreground,
        Some(Color::RGB(RGB(255, 0, 0)))
    );
    assert_eq!(
        theme.elements[&Element::Level].foreground,
        Some(Color::Plain(PlainColor::Red))
    );
    assert_eq!(
        theme.elements[&Element::Time].foreground,
        Some(Color::Plain(PlainColor::BrightBlue))
    );
    assert_eq!(theme.elements[&Element::Caller].foreground, Some(Color::Palette(42)));
    assert_eq!(theme.elements[&Element::Key].foreground, Some(Color::Palette(0)));
    assert_eq!(theme.elements[&Element::String].foreground, Some(Color::Palette(255)));
    assert_eq!(
        theme.elements[&Element::Logger].foreground,
        Some(Color::Plain(PlainColor::Green))
    );
}

#[test]
fn test_v0_plain_color_case_sensitivity() {
    let theme = theme("v0-color-formats");

    assert_eq!(
        theme.elements[&Element::Level].foreground,
        Some(Color::Plain(PlainColor::Red))
    );
}
