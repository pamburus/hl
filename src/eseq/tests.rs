use super::*;

#[test]
fn test_mode_render() {
    let mut buf = Vec::new();
    Mode::Reset.render(&mut buf);
    assert_eq!(buf, b"0");

    buf.clear();
    Mode::Bold.render(&mut buf);
    assert_eq!(buf, b"1");

    buf.clear();
    Mode::Italic.render(&mut buf);
    assert_eq!(buf, b"3");
}

#[test]
fn test_color_bright() {
    let bright_red1 = Color::Red.bright();
    let bright_red2 = Color::Red.bright();
    let fg_code = bright_red1.fg();
    let bg_code = bright_red2.bg();

    // Test that we get the expected style codes
    assert!(matches!(
        fg_code,
        StyleCode::Foreground(ColorCode::Plain(Color::Red, Brightness::Bright))
    ));
    assert!(matches!(
        bg_code,
        StyleCode::Background(ColorCode::Plain(Color::Red, Brightness::Bright))
    ));
}

#[test]
fn test_color_fg_bg() {
    let fg_code = Color::Blue.fg();
    let bg_code = Color::Green.bg();

    assert!(matches!(
        fg_code,
        StyleCode::Foreground(ColorCode::Plain(Color::Blue, Brightness::Normal))
    ));
    assert!(matches!(
        bg_code,
        StyleCode::Background(ColorCode::Plain(Color::Green, Brightness::Normal))
    ));
}

#[test]
fn test_color_render() {
    let mut buf = Vec::new();
    Color::Red.render(&mut buf, 30);
    assert_eq!(buf, b"31");

    buf.clear();
    Color::Blue.render(&mut buf, 40);
    assert_eq!(buf, b"44");
}

#[test]
fn test_plain_color_fg_bg() {
    let plain1 = Color::Cyan.bright();
    let plain2 = Color::Cyan.bright();
    let fg_code = plain1.fg();
    let bg_code = plain2.bg();

    assert!(matches!(
        fg_code,
        StyleCode::Foreground(ColorCode::Plain(Color::Cyan, Brightness::Bright))
    ));
    assert!(matches!(
        bg_code,
        StyleCode::Background(ColorCode::Plain(Color::Cyan, Brightness::Bright))
    ));
}

#[test]
fn test_color_code_render() {
    let mut buf = Vec::new();

    // Test Default
    ColorCode::Default.render(&mut buf, 30);
    assert_eq!(buf, b"39");

    buf.clear();
    ColorCode::Plain(Color::Red, Brightness::Normal).render(&mut buf, 30);
    assert_eq!(buf, b"31");

    buf.clear();
    ColorCode::Plain(Color::Red, Brightness::Bright).render(&mut buf, 30);
    assert_eq!(buf, b"91");

    buf.clear();
    ColorCode::Palette(42).render(&mut buf, 30);
    assert_eq!(buf, b"38;5;42");

    buf.clear();
    ColorCode::Rgb(255, 128, 64).render(&mut buf, 30);
    assert_eq!(buf, b"38;2;255;128;64");
}

#[test]
fn test_style_code_render() {
    let mut buf = Vec::new();

    StyleCode::Mode(Mode::Bold).render(&mut buf);
    assert_eq!(buf, b"1");

    buf.clear();
    StyleCode::Foreground(ColorCode::Plain(Color::Red, Brightness::Normal)).render(&mut buf);
    assert_eq!(buf, b"31");

    buf.clear();
    StyleCode::Background(ColorCode::Plain(Color::Blue, Brightness::Normal)).render(&mut buf);
    assert_eq!(buf, b"44");
}

#[test]
fn test_style_code_from_mode() {
    let style: StyleCode = Mode::Bold.into();
    assert!(matches!(style, StyleCode::Mode(Mode::Bold)));
}

#[test]
fn test_sequence_reset() {
    let seq = Sequence::reset();
    assert_eq!(seq.data(), b"\x1b[0m");
}

#[test]
fn test_sequence_from_style_code() {
    let style = StyleCode::Mode(Mode::Bold);
    let seq: Sequence = style.into();
    assert_eq!(seq.data(), b"\x1b[0;1m");
}

#[test]
fn test_sequence_from_two_style_codes() {
    let style1 = StyleCode::Mode(Mode::Bold);
    let style2 = StyleCode::Foreground(ColorCode::Plain(Color::Red, Brightness::Normal));
    let seq: Sequence = (style1, style2).into();
    assert_eq!(seq.data(), b"\x1b[0;1;31m");
}

#[test]
fn test_sequence_from_three_style_codes() {
    let style1 = StyleCode::Mode(Mode::Bold);
    let style2 = StyleCode::Foreground(ColorCode::Plain(Color::Red, Brightness::Normal));
    let style3 = StyleCode::Background(ColorCode::Plain(Color::Blue, Brightness::Normal));
    let seq: Sequence = (style1, style2, style3).into();
    assert_eq!(seq.data(), b"\x1b[0;1;31;44m");
}

#[test]
fn test_sequence_from_vec_style_codes() {
    let styles = vec![
        StyleCode::Mode(Mode::Bold),
        StyleCode::Foreground(ColorCode::Plain(Color::Green, Brightness::Normal)),
    ];
    let seq: Sequence = styles.into();
    assert_eq!(seq.data(), b"\x1b[0;1;32m");
}

#[test]
fn test_sequence_from_vec_u8() {
    let buf = b"\x1b[0;1;32m".to_vec();
    let seq: Sequence = buf.into();
    assert_eq!(seq.data(), b"\x1b[0;1;32m");
}

#[test]
fn test_sequence_equality() {
    let seq1 = Sequence::reset();
    let seq2 = Sequence::reset();
    assert_eq!(seq1, seq2);

    let style = StyleCode::Mode(Mode::Bold);
    let seq3: Sequence = style.into();
    assert_ne!(seq1, seq3);
}

#[test]
fn test_color_code_fg_bg() {
    let color1 = ColorCode::Plain(Color::Yellow, Brightness::Bright);
    let color2 = ColorCode::Plain(Color::Yellow, Brightness::Bright);
    let fg = color1.fg();
    let bg = color2.bg();

    assert!(matches!(
        fg,
        StyleCode::Foreground(ColorCode::Plain(Color::Yellow, Brightness::Bright))
    ));
    assert!(matches!(
        bg,
        StyleCode::Background(ColorCode::Plain(Color::Yellow, Brightness::Bright))
    ));
}
