use std::io::Write;

// ---

#[repr(u8)]
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Mode {
    Reset = 0,
    Bold,
    Faint,
    Italic,
    Underline,
    SlowBlink,
    RapidBlink,
    Reverse,
    Conceal,
    CrossedOut,
}

impl Mode {
    fn render(&self, buf: &mut Vec<u8>) {
        write!(buf, "{}", (*self as u8)).unwrap()
    }
}

// ---

#[repr(u8)]
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

#[allow(dead_code)]
impl Color {
    pub fn bright(self) -> PlainColor {
        PlainColor(self, Brightness::Bright)
    }

    pub fn fg(self) -> StyleCode {
        ColorCode::Plain(self, Brightness::Normal).fg()
    }

    pub fn bg(self) -> StyleCode {
        ColorCode::Plain(self, Brightness::Normal).bg()
    }

    fn render(&self, buf: &mut Vec<u8>, base: u8) {
        write!(buf, "{}", base + (*self as u8)).unwrap()
    }
}

// ---

pub struct PlainColor(Color, Brightness);

#[allow(dead_code)]
impl PlainColor {
    pub fn fg(self) -> StyleCode {
        ColorCode::Plain(self.0, self.1).fg()
    }

    pub fn bg(self) -> StyleCode {
        ColorCode::Plain(self.0, self.1).bg()
    }
}

// ---

#[allow(dead_code)]
pub enum ColorCode {
    Default,
    Plain(Color, Brightness),
    Palette(u8),
    Rgb(u8, u8, u8),
}

impl ColorCode {
    pub fn fg(self) -> StyleCode {
        StyleCode::Foreground(self)
    }

    pub fn bg(self) -> StyleCode {
        StyleCode::Background(self)
    }

    fn render(&self, buf: &mut Vec<u8>, base: u8) {
        match self {
            Self::Default => write!(buf, "{}", base + 9).unwrap(),
            Self::Plain(color, Brightness::Normal) => color.render(buf, base),
            Self::Plain(color, Brightness::Bright) => color.render(buf, base + 60),
            Self::Palette(color) => write!(buf, "{};5;{}", base + 8, color).unwrap(),
            Self::Rgb(r, g, b) => write!(buf, "{};2;{};{};{}", base + 8, r, g, b).unwrap(),
        }
    }
}

// ---

#[allow(dead_code)]
pub enum Brightness {
    Normal,
    Bright,
}

// ---

#[allow(dead_code)]
pub enum StyleCode {
    Mode(Mode),
    Background(ColorCode),
    Foreground(ColorCode),
}

impl StyleCode {
    fn render(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Mode(mode) => mode.render(buf),
            Self::Background(color) => color.render(buf, 40),
            Self::Foreground(color) => color.render(buf, 30),
        }
    }
}

impl From<Mode> for StyleCode {
    fn from(mode: Mode) -> Self {
        StyleCode::Mode(mode)
    }
}

// ---

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Sequence {
    buf: Vec<u8>,
}

impl Sequence {
    pub fn reset() -> Self {
        let mut buf = Vec::with_capacity(5);
        begin(&mut buf);
        end(&mut buf);
        Self { buf }
    }

    pub fn data(&self) -> &[u8] {
        &self.buf
    }
}

impl From<Vec<u8>> for Sequence {
    fn from(buf: Vec<u8>) -> Self {
        Self { buf }
    }
}

impl From<StyleCode> for Sequence {
    fn from(c1: StyleCode) -> Self {
        let mut buf = Vec::with_capacity(24);
        begin(&mut buf);
        next(&mut buf);
        c1.render(&mut buf);
        end(&mut buf);
        Self { buf }
    }
}

impl From<(StyleCode, StyleCode)> for Sequence {
    fn from(v: (StyleCode, StyleCode)) -> Self {
        let mut buf = Vec::with_capacity(48);
        begin(&mut buf);
        next(&mut buf);
        v.0.render(&mut buf);
        next(&mut buf);
        v.1.render(&mut buf);
        end(&mut buf);
        Self { buf }
    }
}

impl From<(StyleCode, StyleCode, StyleCode)> for Sequence {
    fn from(v: (StyleCode, StyleCode, StyleCode)) -> Self {
        let mut buf = Vec::with_capacity(72);
        begin(&mut buf);
        next(&mut buf);
        v.0.render(&mut buf);
        next(&mut buf);
        v.1.render(&mut buf);
        next(&mut buf);
        v.2.render(&mut buf);
        end(&mut buf);
        Self { buf }
    }
}

impl From<Vec<StyleCode>> for Sequence {
    fn from(v: Vec<StyleCode>) -> Self {
        let mut buf = Vec::new();
        begin(&mut buf);
        for s in v {
            next(&mut buf);
            s.render(&mut buf);
        }
        end(&mut buf);
        Self { buf }
    }
}

// ---

#[inline]
fn begin(buf: &mut Vec<u8>) {
    buf.push(b'\x1b');
    buf.push(b'[');
    buf.push(b'0');
}

#[inline]
fn next(buf: &mut Vec<u8>) {
    buf.push(b';');
}

#[inline]
fn end(buf: &mut Vec<u8>) {
    buf.push(b'm');
}

#[cfg(test)]
mod tests {
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
        match fg_code {
            StyleCode::Foreground(ColorCode::Plain(Color::Red, Brightness::Bright)) => {}
            _ => panic!("Expected bright red foreground"),
        }

        match bg_code {
            StyleCode::Background(ColorCode::Plain(Color::Red, Brightness::Bright)) => {}
            _ => panic!("Expected bright red background"),
        }
    }

    #[test]
    fn test_color_fg_bg() {
        let fg_code = Color::Blue.fg();
        let bg_code = Color::Green.bg();

        match fg_code {
            StyleCode::Foreground(ColorCode::Plain(Color::Blue, Brightness::Normal)) => {}
            _ => panic!("Expected blue foreground"),
        }

        match bg_code {
            StyleCode::Background(ColorCode::Plain(Color::Green, Brightness::Normal)) => {}
            _ => panic!("Expected green background"),
        }
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

        match fg_code {
            StyleCode::Foreground(ColorCode::Plain(Color::Cyan, Brightness::Bright)) => {}
            _ => panic!("Expected bright cyan foreground"),
        }

        match bg_code {
            StyleCode::Background(ColorCode::Plain(Color::Cyan, Brightness::Bright)) => {}
            _ => panic!("Expected bright cyan background"),
        }
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
        match style {
            StyleCode::Mode(Mode::Bold) => {}
            _ => panic!("Expected Mode::Bold"),
        }
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

        match fg {
            StyleCode::Foreground(ColorCode::Plain(Color::Yellow, Brightness::Bright)) => {}
            _ => panic!("Expected yellow bright foreground"),
        }

        match bg {
            StyleCode::Background(ColorCode::Plain(Color::Yellow, Brightness::Bright)) => {}
            _ => panic!("Expected yellow bright background"),
        }
    }
}
