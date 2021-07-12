use std::collections::HashMap;
use std::vec::Vec;

use crate::eseq;
use crate::settings;
use crate::types;

use eseq::{Brightness, Color, ColorCode, Mode, Sequence, StyleCode};
pub use types::Level;

#[repr(u8)]
pub enum Element {
    Time,
    Level,
    Logger,
    Caller,
    Message,
    EqualSign,
    Brace,
    Quote,
    Delimiter,
    Comma,
    AtSign,
    Ellipsis,
    FieldKey,
    Null,
    Boolean,
    Number,
    String,
    Whitespace,
}

pub type Buf = Vec<u8>;

pub struct Styler<'a> {
    pack: &'a StylePack,
    current: Option<usize>,
}

pub struct Theme {
    packs: HashMap<Level, StylePack>,
    default: StylePack,
}

#[derive(Clone, Eq, PartialEq)]
struct Style(Sequence);

impl Style {
    pub fn apply(&self, buf: &mut Buf) {
        buf.extend_from_slice(self.0.data())
    }

    pub fn reset() -> Self {
        Sequence::reset().into()
    }

    fn convert_color(color: &settings::Color) -> ColorCode {
        match color {
            settings::Color::Plain(color) => {
                let c = match color {
                    settings::PlainColor::Black => (Color::Black, Brightness::Normal),
                    settings::PlainColor::Blue => (Color::Blue, Brightness::Normal),
                    settings::PlainColor::Cyan => (Color::Cyan, Brightness::Normal),
                    settings::PlainColor::Green => (Color::Green, Brightness::Normal),
                    settings::PlainColor::Magenta => (Color::Magenta, Brightness::Normal),
                    settings::PlainColor::Red => (Color::Red, Brightness::Normal),
                    settings::PlainColor::White => (Color::White, Brightness::Normal),
                    settings::PlainColor::Yellow => (Color::Yellow, Brightness::Normal),
                    settings::PlainColor::BrightBlack => (Color::Black, Brightness::Bright),
                    settings::PlainColor::BrightBlue => (Color::Blue, Brightness::Bright),
                    settings::PlainColor::BrightCyan => (Color::Cyan, Brightness::Bright),
                    settings::PlainColor::BrightGreen => (Color::Green, Brightness::Bright),
                    settings::PlainColor::BrightMagenta => (Color::Magenta, Brightness::Bright),
                    settings::PlainColor::BrightRed => (Color::Red, Brightness::Bright),
                    settings::PlainColor::BrightWhite => (Color::White, Brightness::Bright),
                    settings::PlainColor::BrightYellow => (Color::Yellow, Brightness::Bright),
                };
                ColorCode::Plain(c.0, c.1)
            }
            settings::Color::Palette(code) => ColorCode::Palette(*code),
            settings::Color::RGB(settings::RGB(r, g, b)) => ColorCode::RGB(*r, *g, *b),
        }
    }
}

impl<T: Into<Sequence>> From<T> for Style {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl From<&settings::Style> for Style {
    fn from(style: &settings::Style) -> Self {
        let mut codes = Vec::<StyleCode>::new();
        for mode in &style.modes {
            codes.push(
                match mode {
                    settings::Mode::Bold => Mode::Bold,
                    settings::Mode::Conseal => Mode::Conseal,
                    settings::Mode::CrossedOut => Mode::CrossedOut,
                    settings::Mode::Faint => Mode::Faint,
                    settings::Mode::Italic => Mode::Italic,
                    settings::Mode::RapidBlink => Mode::RapidBlink,
                    settings::Mode::Reverse => Mode::Reverse,
                    settings::Mode::SlowBlink => Mode::SlowBlink,
                    settings::Mode::Underline => Mode::Underline,
                }
                .into(),
            );
        }
        if let Some(color) = &style.background {
            codes.push(StyleCode::Background(Self::convert_color(color)));
        }
        if let Some(color) = &style.foreground {
            codes.push(StyleCode::Foreground(Self::convert_color(color)));
        }
        Self(codes.into())
    }
}

impl<'a> Styler<'a> {
    pub fn set(&mut self, buf: &mut Buf, e: Element) {
        self.set_style(buf, self.pack.elements[e as usize])
    }

    fn reset(&mut self, buf: &mut Buf) {
        self.set_style(buf, None)
    }

    fn set_style(&mut self, buf: &mut Buf, style: Option<usize>) {
        let style = match style {
            Some(style) => Some(style),
            None => self.pack.reset,
        };
        if let Some(style) = style {
            if self.current != Some(style) {
                self.current = Some(style);
                let style = &self.pack.styles[style];
                style.apply(buf);
            }
        }
    }
}

impl Theme {
    pub fn apply<'a, F: FnOnce(&mut Buf, &mut Styler<'a>)>(
        &'a self,
        buf: &mut Buf,
        level: &Option<Level>,
        f: F,
    ) {
        let mut styler = Styler {
            pack: match level {
                Some(level) => match self.packs.get(level) {
                    Some(pack) => pack,
                    None => &self.default,
                },
                None => &self.default,
            },
            current: None,
        };
        f(buf, &mut styler);
        styler.reset(buf)
    }
}

struct StylePack {
    elements: Vec<Option<usize>>,
    reset: Option<usize>,
    styles: Vec<Style>,
}

impl StylePack {
    fn new() -> Self {
        Self {
            styles: vec![Style::reset()],
            reset: Some(0),
            elements: vec![None; 255],
        }
    }

    fn none() -> Self {
        Self {
            elements: vec![None; 255],
            reset: None,
            styles: Vec::new(),
        }
    }

    fn add(&mut self, element: Element, style: &Style) {
        let pos = match self.styles.iter().position(|x| x == style) {
            Some(pos) => pos,
            None => {
                self.styles.push(style.clone());
                self.styles.len() - 1
            }
        };
        self.elements[element as usize] = Some(pos);
    }

    fn load(s: &settings::StylePack<settings::Style>) -> Self {
        let mut result = Self::new();
        result.add(Element::Caller, &Style::from(&s.caller));
        result.add(Element::Comma, &Style::from(&s.comma));
        result.add(Element::Delimiter, &Style::from(&s.delimiter));
        result.add(Element::Ellipsis, &Style::from(&s.ellipsis));
        result.add(Element::EqualSign, &Style::from(&s.equal_sign));
        result.add(Element::FieldKey, &Style::from(&s.field_key));
        result.add(Element::Level, &Style::from(&s.level));
        result.add(Element::Boolean, &Style::from(&s.boolean));
        result.add(Element::Null, &Style::from(&s.null));
        result.add(Element::Number, &Style::from(&s.number));
        result.add(Element::String, &Style::from(&s.string));
        result.add(Element::AtSign, &Style::from(&s.at_sign));
        result.add(Element::Logger, &Style::from(&s.logger));
        result.add(Element::Message, &Style::from(&s.message));
        result.add(Element::Quote, &Style::from(&s.quote));
        result.add(Element::Time, &Style::from(&s.time));
        result.add(Element::Whitespace, &Style::from(&s.time));
        result
    }
}

impl Theme {
    pub fn none() -> Self {
        Self {
            packs: HashMap::new(),
            default: StylePack::none(),
        }
    }

    pub fn load(s: &settings::Theme) -> Self {
        let default = StylePack::load(&s.default);
        let mut packs = HashMap::new();
        for (level, pack) in &s.levels {
            packs.insert(*level, StylePack::load(&s.default.clone().merged(&pack)));
        }
        Self { default, packs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme() {
        let theme = Theme::none();
        let mut buf = Vec::new();
        theme.apply(&mut buf, &Some(Level::Debug), |buf, styler| {
            styler.set(buf, Element::Message);
        });
    }
}
