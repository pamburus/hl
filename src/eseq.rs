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
    Conseal,
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
    RGB(u8, u8, u8),
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
            Self::RGB(r, g, b) => write!(buf, "{};2;{};{};{}", base + 8, r, g, b).unwrap(),
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

#[derive(Clone, Eq, PartialEq)]
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
