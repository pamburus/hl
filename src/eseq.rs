use std::io::Write;

#[repr(u8)]
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Style {
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

pub struct PlainColor(Color, Brightness);

#[allow(dead_code)]
pub enum ColorCode {
    Plain(Color, Brightness),
    Palette(u8),
    RGB(u8, u8, u8),
}

#[allow(dead_code)]
pub enum Brightness {
    Normal,
    Bright,
}

#[allow(dead_code)]
pub enum StyleCode {
    Style(Style),
    Background(ColorCode),
    Foreground(ColorCode),
}

impl StyleCode {
    fn render(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Style(style) => style.render(buf),
            Self::Background(color) => color.render(buf, 40),
            Self::Foreground(color) => color.render(buf, 30),
        }
    }
}

impl From<Style> for StyleCode {
    fn from(style: Style) -> Self {
        StyleCode::Style(style)
    }
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

#[allow(dead_code)]
impl PlainColor {
    pub fn fg(self) -> StyleCode {
        ColorCode::Plain(self.0, self.1).fg()
    }

    pub fn bg(self) -> StyleCode {
        ColorCode::Plain(self.0, self.1).bg()
    }
}

impl Style {
    fn render(&self, buf: &mut Vec<u8>) {
        write!(buf, "{}", (*self as u8)).unwrap()
    }
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
            Self::Plain(color, Brightness::Normal) => color.render(buf, base),
            Self::Plain(color, Brightness::Bright) => color.render(buf, base + 60),
            Self::Palette(color) => write!(buf, "{};5;{}", base + 8, color).unwrap(),
            Self::RGB(r, g, b) => write!(buf, "{};2;{};{};{}", base + 8, r, g, b).unwrap(),
        }
    }
}

#[allow(dead_code)]
pub fn eseq0() -> Vec<u8> {
    let mut buf = Vec::with_capacity(5);
    begin(&mut buf);
    end(&mut buf);
    buf
}

#[allow(dead_code)]
pub fn eseq1(c1: StyleCode) -> Vec<u8> {
    let mut buf = Vec::with_capacity(24);
    begin(&mut buf);
    next(&mut buf);
    c1.render(&mut buf);
    end(&mut buf);
    buf
}

#[allow(dead_code)]
pub fn eseq2(c1: StyleCode, c2: StyleCode) -> Vec<u8> {
    let mut buf = Vec::with_capacity(48);
    begin(&mut buf);
    next(&mut buf);
    c1.render(&mut buf);
    next(&mut buf);
    c2.render(&mut buf);
    end(&mut buf);
    buf
}

#[allow(dead_code)]
pub fn eseq3(c1: StyleCode, c2: StyleCode, c3: StyleCode) -> Vec<u8> {
    let mut buf = Vec::with_capacity(72);
    begin(&mut buf);
    next(&mut buf);
    c1.render(&mut buf);
    next(&mut buf);
    c2.render(&mut buf);
    next(&mut buf);
    c3.render(&mut buf);
    end(&mut buf);
    buf
}

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
